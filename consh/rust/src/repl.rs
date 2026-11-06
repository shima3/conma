//! Interactive mode (consh_repl.se): read a command from the standard input
//! line by line until it is complete, parse it and run it.
//!
//!   - The prompt is the value of PS1 ("$ " if it is not defined), and PS2
//!     ("> ") while the rest of a command is read, written to the standard
//!     error as it is.
//!   - An error does not end the shell: the message, $? 1, and the next
//!     prompt. Variables, functions, the current directory and jobs stay.
//!   - At the end of the input, the $( ) that have not finished are ended and
//!     the shell ends with the last $? (it does not wait for the jobs). If a
//!     command is not complete, a syntax error and status 1.
//!   - The script name and $0 are "stdin", and a line number counts the lines
//!     from the start of interactive mode.
//!   - Ctrl-C (SIGINT) does not end the shell. While a command runs, it stops
//!     the command (its external command gets SIGTERM, and its pipelines and
//!     $( ) are stopped; not its jobs), $? is 130 and the next prompt comes.
//!     At a prompt, it drops the part of a command read so far.
//!
//! Each command runs in a thread of its own, so that Ctrl-C can stop it. It
//! shares the variables, functions and jobs with the shell; its current
//! directory becomes the shell's when it ends (not when it is stopped).
//!
//! This is a module of eval, so that it can use the contexts as they are.

use super::*;
use crate::parse;
use std::sync::Condvar;

struct Running {
    ctx: Ctx,
    /// The $( ) of the shell before the command: Ctrl-C stops the others.
    before: Vec<Arc<Subst>>,
    killed: bool,
    done: bool,
    /// A second Ctrl-C while a stopped command has not ended yet: do not wait.
    abandoned: bool,
}

struct State {
    running: Option<Running>,
    /// Ctrl-C at a prompt: drop the command read so far.
    discard: bool,
}

type Shared = Arc<(Mutex<State>, Condvar)>;

pub fn repl_run(args: Vec<String>) -> i32 {
    let sh = Ctx::top_with("stdin", args, true);
    let state: Shared = Arc::new((Mutex::new(State { running: None, discard: false }), Condvar::new()));
    let interrupts = os::trap_interrupt();
    {
        let sh = sh.clone();
        let state = state.clone();
        spawn(move || watch(&sh, &state, interrupts));
    }
    let mut line_no = 1;
    loop {
        let (lines, n) = command(&sh, &state, line_no);
        line_no = n;
        run(&sh, &state, lines);
    }
}

/// Wait for Ctrl-C: stop the command that runs, or drop the command read so far.
fn watch(sh: &Ctx, state: &Shared, mut interrupts: std::fs::File) {
    use std::io::Read;
    let mut b = [0u8; 1];
    while let Ok(1) = interrupts.read(&mut b) {
        let mut g = state.0.lock().unwrap();
        match &mut g.running {
            None => {
                g.discard = true;
                sh.set_status("130");
                os::print_error("\n");
                prompt(sh, "PS1", "$ ");
            }
            Some(r) if !r.killed => {
                r.killed = true;
                stop(sh, r);
            }
            Some(r) => {
                r.abandoned = true;
                state.1.notify_all();
            }
        }
    }
}

/// Stop the command: its thread, the external command it runs, the
/// pipeline it waits for, and the $( ) it has started.
fn stop(sh: &Ctx, r: &Running) {
    r.ctx.killed.store(true, Ordering::SeqCst);
    if let Some(pid) = *r.ctx.proc_ref.lock().unwrap() {
        os::terminate(pid);
    }
    let members = std::mem::take(&mut *r.ctx.pipe_members.lock().unwrap());
    for m in &members {
        vproc_stop(m);
    }
    let substs = sh.global.substs.lock().unwrap().clone();
    for rec in substs {
        if r.before.iter().any(|b| Arc::ptr_eq(b, &rec)) {
            continue;
        }
        match &rec.kind {
            SubstKind::Chan(c) => c.cancel(),
            SubstKind::Os(st) => st.cancelled.store(true, Ordering::SeqCst),
        }
        vproc_stop(&rec.member);
    }
}

/// Read lines until a command is complete: its command lines and the number
/// of the next line. A syntax error: the message, $? 1 and no command lines.
fn command(sh: &Ctx, state: &Shared, mut line_no: i64) -> (Vec<Sexp>, i64) {
    let mut text = String::new();
    let mut start = line_no;
    loop {
        let first = text.is_empty();
        if first {
            prompt(sh, "PS1", "$ ");
        } else {
            prompt(sh, "PS2", "> ");
        }
        let line = read_line(sh);
        {
            // Ctrl-C at the prompt: start again from this line.
            let mut g = state.0.lock().unwrap();
            if g.discard {
                g.discard = false;
                text.clear();
                start = line_no;
            }
        }
        let line = match line {
            None => end(sh, &text, start),
            Some(l) => l,
        };
        text.push_str(&line);
        text.push('\n');
        line_no += 1;
        // A \ at the end of a line continues it.
        if text.ends_with("\\\n") {
            continue;
        }
        match parse::parse(&text, start) {
            Ok(lines) => return (lines, line_no),
            Err((_, msg)) if parse::incomplete(&msg) => continue,
            Err((full, _)) => {
                os::print_error(&format!("consh: stdin:{}\n", full));
                sh.set_status("1");
                return (Vec::new(), line_no);
            }
        }
    }
}

/// The end of the input: a syntax error if a command is not complete, or
/// the shell ends with $?.
fn end(sh: &Ctx, text: &str, start: i64) -> ! {
    if !text.is_empty() {
        if let Err((full, _)) = parse::parse(text, start) {
            os::print_error(&format!("consh: stdin:{}\n", full));
            std::process::exit(1);
        }
    }
    subst_finish(sh);
    std::process::exit(sh.status().parse().unwrap_or(0));
}

/// Write the value of the variable name (fallback if it is not defined) to
/// the standard error.
fn prompt(sh: &Ctx, name: &str, fallback: &str) {
    let text = match sh.defined(None, name) {
        Ok(true) => match sh.lookup(None, name).and_then(|v| expand::value_to_string(sh, None, &v)) {
            Ok(s) => s,
            Err(_) => fallback.to_string(),
        },
        _ => fallback.to_string(),
    };
    os::print_error(&text);
}

/// One line of the shell's standard input (without its newline), moving its
/// position; None at the end. The input stays "untouched", so external
/// commands get the Stream itself.
fn read_line(sh: &Ctx) -> Option<String> {
    let list = sh.input.list.lock().unwrap().clone();
    match list.uncons() {
        Ok(Some((v, rest))) => {
            *sh.input.list.lock().unwrap() = rest;
            Some(expand::value_to_string(sh, None, &v).unwrap_or_default())
        }
        _ => None,
    }
}

/// Run the command lines in another thread and wait for them. If Ctrl-C
/// stopped them, $? is 130; otherwise the shell moves to their directory.
fn run(sh: &Ctx, state: &Shared, lines: Vec<Sexp>) {
    if lines.is_empty() {
        return;
    }
    let ctx = Ctx {
        killed: Arc::new(AtomicBool::new(false)),
        proc_ref: Arc::new(Mutex::new(None)),
        pipe_members: Arc::new(Mutex::new(Vec::new())),
        cwd: Arc::new(Mutex::new(sh.cwd())),
        // The resets of the command (a continuation of an earlier command is
        // from another thread, and cannot be called).
        meta: Arc::new(Mutex::new(Vec::new())),
        ..sh.clone()
    };
    let before = sh.global.substs.lock().unwrap().clone();
    state.0.lock().unwrap().running = Some(Running { ctx: ctx.clone(), before, killed: false, done: false, abandoned: false });
    {
        let ctx = ctx.clone();
        let state = state.clone();
        spawn(move || {
            if let Err(Flow::Exit(code)) = eval_lines(&ctx, &lines) {
                subst_finish(&ctx);
                std::process::exit(code);
            }
            let mut g = state.0.lock().unwrap();
            if let Some(r) = &mut g.running {
                r.done = true;
            }
            state.1.notify_all();
        });
    }
    let mut g = state.0.lock().unwrap();
    while !g.running.as_ref().map(|r| r.done || r.abandoned).unwrap_or(true) {
        g = state.1.wait(g).unwrap();
    }
    let r = g.running.take().unwrap();
    drop(g);
    if r.killed {
        sh.set_status("130");
        os::print_error("\n");
        return;
    }
    *sh.cwd.lock().unwrap() = ctx.cwd();
}
