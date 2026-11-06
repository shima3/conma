//! Running statements, as consh_eval.se: contexts, variables, functions,
//! if and for statements, simple commands, builtins, redirections and
//! external commands.
//!
//! A context (Ctx, the Dict sh of the ConMa version) is cheap to copy: the
//! tables it shares with the copies (local variables, functions, the
//! environment) are behind Arcs. The ConMa version leaves a function, a loop
//! or the shell with a continuation; here they are Err(Flow) values that the
//! function call, the loop or main catches.

use crate::ast::{Pos, Sexp};
use crate::chan::{self, Chan, Writer};
use crate::expand::{self, Limit};
use crate::os::{self, Fd};
use crate::value::{Flow, LazyList, Value, R};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[path = "repl.rs"]
mod repl;
pub use repl::repl_run;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn new_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

// ── Contexts ──

/// What a VProc of the ConMa version keeps for itself: $?, $! and whether the
/// shell is interactive.
pub struct Global {
    pub status: Mutex<String>,
    pub last_bg: Mutex<String>,
    pub interactive: bool,
    /// The $( ) started by this thread that have not finished (newest first).
    substs: Mutex<Vec<Arc<Subst>>>,
    /// The jobs started by this thread with & (newest first).
    jobs: Mutex<Vec<Arc<Job>>>,
}

/// The number of elements a writer of a channel may write before the reader
/// takes them, and that the end of a statement reads ahead of a $( ) by an
/// OS pipe.
const PIPE_CAP: usize = 16;

/// Run f in a new thread with a large stack (the evaluation recurses).
pub fn spawn<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> std::thread::JoinHandle<T> {
    std::thread::Builder::new().stack_size(256 << 20).spawn(f).expect("cannot start a thread")
}

/// A local variable: a value, or the value that a variable list of a job
/// (f > (a b) &) has not received yet (Value_force waits for it).
#[derive(Clone)]
pub enum Var {
    V(Value),
    P(Arc<Promise>),
}

pub struct Promise {
    cell: Mutex<Option<Value>>,
    changed: std::sync::Condvar,
}

impl Promise {
    fn new() -> Arc<Promise> {
        Arc::new(Promise { cell: Mutex::new(None), changed: std::sync::Condvar::new() })
    }
    /// Set the value (a second time does nothing).
    fn resolve(&self, v: Value) {
        let mut c = self.cell.lock().unwrap();
        if c.is_none() {
            *c = Some(v);
            self.changed.notify_all();
        }
    }
    fn wait(&self) -> Value {
        let mut c = self.cell.lock().unwrap();
        loop {
            if let Some(v) = &*c {
                return v.clone();
            }
            c = self.changed.wait(c).unwrap();
        }
    }
}

pub type Vars = Arc<Mutex<HashMap<String, Var>>>;
type Layer = Arc<Mutex<HashMap<String, String>>>;

/// The environment: the variables of the whole shell (base) and the layers
/// of the variable definitions before commands (innermost first).
#[derive(Clone)]
pub struct Env {
    base: Layer,
    layers: Arc<Vec<Layer>>,
}

impl Env {
    /// A copy: export in it does not change this environment.
    fn copy(&self) -> Env {
        let base = Arc::new(Mutex::new(self.base.lock().unwrap().clone()));
        let layers = self.layers.iter().map(|l| Arc::new(Mutex::new(l.lock().unwrap().clone()))).collect();
        Env { base, layers: Arc::new(layers) }
    }
    fn lookup(&self, name: &str) -> Option<String> {
        for l in self.layers.iter() {
            if let Some(v) = l.lock().unwrap().get(name) {
                return Some(v.clone());
            }
        }
        self.base.lock().unwrap().get(name).cloned()
    }
    fn push(&self, layer: HashMap<String, String>) -> Env {
        let mut layers = vec![Arc::new(Mutex::new(layer))];
        layers.extend(self.layers.iter().cloned());
        Env { base: self.base.clone(), layers: Arc::new(layers) }
    }
    /// export: the value in the base and in every layer that has the name.
    fn export(&self, name: &str, value: &str) {
        self.base.lock().unwrap().insert(name.to_string(), value.to_string());
        for l in self.layers.iter() {
            let mut l = l.lock().unwrap();
            if l.contains_key(name) {
                l.insert(name.to_string(), value.to_string());
            }
        }
    }
    /// The variables for an external command: the base with the layers on
    /// it, from the outermost.
    fn dict(&self) -> HashMap<String, String> {
        let mut d = self.base.lock().unwrap().clone();
        for l in self.layers.iter().rev() {
            for (k, v) in l.lock().unwrap().iter() {
                d.insert(k.clone(), v.clone());
            }
        }
        d
    }
}

/// The functions defined in a scope, and the lexically enclosing scope.
pub struct Scope {
    funcs: Mutex<HashMap<String, Arc<Func>>>,
    parent: Option<Arc<Scope>>,
}

pub struct Func {
    names: Vec<String>,
    rest: String,
    body: Sexp,
    scope: Arc<Scope>,
    /// The local variables when the function was defined.
    vars: HashMap<String, Var>,
}

/// The for loops being run, innermost first: the name of the loop variable
/// and the id that break and continue carry.
struct LoopNode {
    name: String,
    id: u64,
    next: Option<Arc<LoopNode>>,
}

/// Where standard output and standard error go. Targets copied with 2>&1 or
/// 1>&2 have the same id.
#[derive(Clone)]
pub struct Target {
    pub id: u64,
    pub kind: TKind,
}

#[derive(Clone)]
pub enum TKind {
    /// The standard output of the shell; LazyLists are written with the OFS
    /// of the top-level context.
    Top(Arc<Ctx>),
    /// A Stream: each element with a newline after it, a LazyList with the
    /// OFS of the context that writes it.
    Stream(Arc<Fd>),
    /// A Stream passed to the body of a function, an if or a for statement:
    /// LazyLists with the OFS of the context of the redirection.
    File(Arc<Fd>, Arc<Ctx>, Pos),
    /// The OS pipe to the next element of a pipeline, written by an element
    /// that is not one external command: LazyLists with the OFS of the
    /// pipeline's context. If the reader has closed it, the element ends.
    Pipe(Arc<Fd>, Arc<Ctx>),
    /// A writer of a channel, used only by the thread that made it; another
    /// thread gets a new writer of the same channel (child_target).
    Chan(Arc<Writer>),
    /// A variable list (> (a b . rest)): elements go to the variables in turn.
    Capture(Arc<Record>),
}

/// Standard input. list holds the elements not read yet, shared by the
/// copies; stream is the file it was made from, given as it is to an
/// external command until list has been read.
pub struct Input {
    list: Mutex<LazyList>,
    stream: Option<Arc<Fd>>,
    touched: AtomicBool,
    /// The context whose OFS makes LazyList elements Strings (made from a
    /// list); an input from a Stream has only Strings.
    sh: Option<Arc<Ctx>>,
}

impl Input {
    fn from_list(sh: &Ctx, list: LazyList) -> Arc<Input> {
        Arc::new(Input { list: Mutex::new(list), stream: None, touched: AtomicBool::new(true), sh: Some(Arc::new(sh.clone())) })
    }
    fn from_stream(fd: Arc<Fd>) -> Arc<Input> {
        Arc::new(Input { list: Mutex::new(lines_of(fd.clone())), stream: Some(fd), touched: AtomicBool::new(false), sh: None })
    }
}

#[derive(Clone)]
pub struct Ctx {
    pub global: Arc<Global>,
    pub script: Arc<str>,
    args: Arc<Vec<Value>>,
    vars: Vars,
    scope: Arc<Scope>,
    in_function: bool,
    ret: Option<u64>,
    no_return: Option<Arc<str>>,
    loops: Option<Arc<LoopNode>>,
    env: Env,
    input: Arc<Input>,
    pub out: Target,
    pub err: Target,
    cwd: Arc<Mutex<PathBuf>>,
    /// The id that Sh_abort ends: the thread, or the command whose variable
    /// list is full. None in the thread of the shell.
    abort: Option<u64>,
    /// What a thread (a VProc of the ConMa version) shares with its copies:
    /// whether kill has stopped it, the external command it is running, the
    /// elements of the pipeline it waits for, and the channel of its input
    /// to cancel when it ends.
    killed: Arc<AtomicBool>,
    proc_ref: Arc<Mutex<Option<u32>>>,
    pipe_members: Arc<Mutex<Vec<Member>>>,
    cancel_in: Option<Arc<Chan>>,
    /// The values for the variable lists of a job's simple command (Bg_captures),
    /// taken in order by its redirections.
    bg_captures: Option<Arc<Mutex<std::collections::VecDeque<Arc<BgSpec>>>>>,
    /// In a job of interactive mode: external commands start outside the
    /// foreground process group of the terminal (Ctrl-C does not reach them).
    detached: bool,
    /// In a statement with redirections (a shift there is an error), and the
    /// resets of the thread being run (stage 20, the "meta" of the ConMa version).
    redirected: bool,
    meta: Meta,
}

/// The resets (and calls of continuations) being run in a thread, innermost
/// last: where each goes on when it ends, and the context and the standard
/// input and outputs of its place (for the body of a shift).
type Meta = Arc<Mutex<Vec<ResetInfo>>>;

#[derive(Clone)]
struct ResetInfo {
    k: Chain,
    sh: Ctx,
    input: Arc<Input>,
    out: Target,
    err: Target,
}

impl Ctx {
    pub fn top(script: &str, args: Vec<String>) -> Ctx {
        Ctx::top_with(script, args, false)
    }

    pub fn top_with(script: &str, args: Vec<String>, interactive: bool) -> Ctx {
        let mut base: HashMap<String, String> = std::env::vars_os()
            .map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned()))
            .collect();
        base.remove("CONSH_RLWRAPPED");
        let env = Env { base: Arc::new(Mutex::new(base)), layers: Arc::new(Vec::new()) };
        let global = Arc::new(Global {
            status: Mutex::new("0".into()),
            last_bg: Mutex::new(String::new()),
            interactive,
            substs: Mutex::new(Vec::new()),
            jobs: Mutex::new(Vec::new()),
        });
        let input = Input::from_stream(Fd::standard(0));
        let err = Target { id: new_id(), kind: TKind::Stream(Fd::standard(2)) };
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        let mut sh = Ctx {
            global,
            script: Arc::from(script),
            args: Arc::new(args.into_iter().map(Value::string).collect()),
            vars: Arc::new(Mutex::new(HashMap::new())),
            scope: Arc::new(Scope { funcs: Mutex::new(HashMap::new()), parent: None }),
            in_function: false,
            ret: None,
            no_return: None,
            loops: None,
            env,
            input,
            out: err.clone(),
            err,
            cwd: Arc::new(Mutex::new(cwd)),
            abort: None,
            killed: Arc::new(AtomicBool::new(false)),
            proc_ref: Arc::new(Mutex::new(None)),
            pipe_members: Arc::new(Mutex::new(Vec::new())),
            cancel_in: None,
            bg_captures: None,
            detached: false,
            redirected: false,
            meta: Arc::new(Mutex::new(Vec::new())),
        };
        // The standard output writes with the OFS of this context; the
        // Target holds a copy whose own out is the standard error.
        let top = Arc::new(sh.clone());
        sh.out = Target { id: new_id(), kind: TKind::Top(top) };
        sh
    }

    /// A local variable, waiting for the value of a job's variable list.
    fn get_var(&self, name: &str) -> Option<Value> {
        let v = self.vars.lock().unwrap().get(name).cloned();
        match v {
            None => None,
            Some(Var::V(v)) => Some(v),
            Some(Var::P(p)) => Some(p.wait()),
        }
    }

    fn set_var(&self, name: &str, v: Value) {
        self.vars.lock().unwrap().insert(name.to_string(), Var::V(v));
    }

    pub fn cwd(&self) -> PathBuf {
        self.cwd.lock().unwrap().clone()
    }

    fn status(&self) -> String {
        self.global.status.lock().unwrap().clone()
    }

    pub fn set_status(&self, s: &str) {
        *self.global.status.lock().unwrap() = s.to_string();
    }

    /// "consh: script:line:col: msg", or "consh: script: msg" without a position.
    fn message(&self, pos: Pos, msg: &str) -> String {
        match pos {
            None => format!("consh: {}: {}\n", self.script, msg),
            Some((l, c)) => format!("consh: {}:{}:{}: {}\n", self.script, l, c, msg),
        }
    }

    /// An error of the script: the message, and the shell ends with status 1.
    pub fn error(&self, pos: Pos, msg: &str) -> Flow {
        os::print_error(&self.message(pos, msg));
        if !self.global.interactive {
            std::process::exit(1);
        }
        self.set_status("1");
        Flow::Error
    }

    /// An error after which the script goes on (a file that cannot be opened,
    /// a command that cannot be started).
    fn warn(&self, pos: Pos, msg: &str) {
        os::print_error(&self.message(pos, msg));
    }

    fn script_args(&self, pos: Pos) -> R<&Arc<Vec<Value>>> {
        if self.in_function {
            return Err(self.error(pos, "$1, $@ and $# are not defined in a function"));
        }
        Ok(&self.args)
    }

    /// The value of a variable: a local variable, a special name, or an
    /// environment variable. An undefined variable is an error.
    pub fn lookup(&self, pos: Pos, name: &str) -> R<Value> {
        if let Some(v) = self.get_var(name) {
            return Ok(v);
        }
        if name == "@" {
            let args = self.script_args(pos)?;
            return Ok(Value::List(LazyList::from_vec(args.to_vec())));
        }
        if is_digits(name) {
            let n: usize = name.parse().unwrap_or(usize::MAX);
            if n == 0 {
                return Ok(Value::Str(self.script.clone()));
            }
            let args = self.script_args(pos)?;
            return match args.get(n - 1) {
                Some(v) => Ok(v.clone()),
                None => Err(self.error(pos, &format!("missing positional parameter: ${}", name))),
            };
        }
        match name {
            "#" => return Ok(Value::string(self.script_args(pos)?.len().to_string())),
            "?" => return Ok(Value::string(self.status())),
            "!" => return Ok(Value::string(self.global.last_bg.lock().unwrap().clone())),
            "$" => return Ok(Value::string(std::process::id().to_string())),
            _ => {}
        }
        match self.env.lookup(name) {
            Some(v) => Ok(Value::string(v)),
            None => Err(self.error(pos, &format!("undefined variable: {}", name))),
        }
    }

    /// Whether a variable is defined. Positional parameters by the number of
    /// arguments; special variables always.
    pub fn defined(&self, pos: Pos, name: &str) -> R<bool> {
        if is_digits(name) {
            let n: usize = name.parse().unwrap_or(usize::MAX);
            if n == 0 {
                return Ok(true);
            }
            return Ok(self.script_args(pos)?.len() >= n);
        }
        if matches!(name, "@" | "#" | "?" | "!" | "$") {
            return Ok(true);
        }
        if self.vars.lock().unwrap().contains_key(name) {
            return Ok(true);
        }
        Ok(self.env.lookup(name).is_some())
    }

    /// The separator for making a LazyList a String: OFS of the local
    /// variables, then of the environment, else a space.
    pub fn ofs(&self, pos: Pos) -> R<String> {
        let v = self.get_var("OFS");
        match v {
            Some(Value::Str(s)) => Ok(s.to_string()),
            Some(Value::Func(_)) => Err(self.error(pos, expand::FUNC_STRING)),
            Some(Value::List(_)) => Err(self.error(pos, "OFS is a LazyList")),
            None => Ok(self.env.lookup("OFS").unwrap_or_else(|| " ".to_string())),
        }
    }
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// The lines of an input stream without their newlines (\n or \r\n), read
/// as the LazyList is read.
fn lines_of(fd: Arc<Fd>) -> LazyList {
    let reader = Arc::new(Mutex::new(BufReader::new(FdReader(fd))));
    lines_from(reader)
}

struct FdReader(Arc<Fd>);

impl std::io::Read for FdReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut g = self.0.file.lock().unwrap();
        match g.as_mut() {
            Some(f) => std::io::Read::read(f, buf),
            None => Ok(0),
        }
    }
}

fn lines_from(reader: Arc<Mutex<BufReader<FdReader>>>) -> LazyList {
    LazyList::new(move || {
        let mut buf = Vec::new();
        let n = reader.lock().unwrap().read_until(b'\n', &mut buf).unwrap_or(0);
        if n == 0 {
            return Ok(None);
        }
        let line = chomp(&String::from_utf8_lossy(&buf)).to_string();
        Ok(Some((Value::string(line), lines_from(reader))))
    })
}

/// The line without its newline (\n or \r\n).
fn chomp(line: &str) -> &str {
    let line = line.strip_suffix('\n').unwrap_or(line);
    line.strip_suffix('\r').unwrap_or(line)
}

// ── Output ──

/// An element of the output as one line: a LazyList with OFS.
fn line_to_string(sh: &Ctx, pos: Pos, line: &Value) -> R<String> {
    expand::value_to_string(sh, pos, line)
}

fn write_fd(fd: &Fd, s: &str) {
    if let Err(e) = fd.write_all(s.as_bytes()) {
        os::print_error(&format!("__OS_write__: [Errno {}] {}\n", e.raw_os_error().unwrap_or(0), os::strerror(&e)));
        std::process::exit(1);
    }
}

/// Write the element line to a target.
pub fn target_write(sh: &Ctx, pos: Pos, target: &Target, line: &Value) -> R<()> {
    os::jitter();
    match &target.kind {
        TKind::Top(top) => {
            let s = line_to_string(top, None, line)?;
            write_fd(&Fd::standard(1), &format!("{}\n", s));
        }
        TKind::Stream(fd) => {
            let s = line_to_string(sh, pos, line)?;
            write_fd(fd, &format!("{}\n", s));
        }
        TKind::File(fd, fsh, fpos) => {
            let s = line_to_string(fsh, *fpos, line)?;
            write_fd(fd, &format!("{}\n", s));
        }
        TKind::Pipe(fd, psh) => {
            let s = line_to_string(psh, None, line)?;
            if fd.write_all(format!("{}\n", s).as_bytes()).is_err() {
                return Err(sh_abort(sh));
            }
        }
        TKind::Chan(w) => {
            if w.put(line.clone()).is_err() {
                return Err(sh_abort(sh));
            }
        }
        TKind::Capture(r) => capture_put(sh, r, line.clone())?,
    }
    Ok(())
}

/// End the thread, or the command whose variable list is full.
fn sh_abort(sh: &Ctx) -> Flow {
    match sh.abort {
        Some(id) => Flow::Abort(id),
        None => sh.error(None, "the reader of a pipe has finished"),
    }
}

/// The target given to the body of a function, an if or a for statement: a
/// Stream becomes a File with the OFS of sh.
fn target_for_body(sh: &Ctx, pos: Pos, target: &Target) -> Target {
    match &target.kind {
        TKind::Stream(fd) => Target { id: target.id, kind: TKind::File(fd.clone(), Arc::new(sh.clone()), pos) },
        _ => target.clone(),
    }
}

/// The targets for a body: standard error stays as it is unless it goes
/// where standard output goes (2>&1).
fn body_targets(sh: &Ctx, pos: Pos, out: &Target, err: &Target) -> (Target, Target) {
    let o = target_for_body(sh, pos, out);
    let e = if out.id == err.id { o.clone() } else { err.clone() };
    (o, e)
}

// ── Statements ──

pub fn eval_run(script: &str, args: Vec<String>, lines: &[Sexp]) -> i32 {
    let sh = Ctx::top(script, args);
    let r = eval_lines(&sh, lines);
    subst_finish(&sh);
    if let Err(Flow::Exit(code)) = r {
        return code;
    }
    jobs_wait_all(&sh);
    sh.status().parse().unwrap_or(0)
}

// ── The machine ──
// Statements run in continuation-passing style, as in the ConMa version: the
// rest of the computation of a thread is a persistent list of frames (a
// Chain, the CChain of the ConMa version), run by a trampoline (run). A
// statement either finishes at once or pushes the frames that do the rest
// (the statements of a script, the parts of an if, the next item of a for,
// the end of a function call). return, break and continue are Err values
// that run unwinds to their frame. A statement with redirections runs its
// body in a nested run, so that the redirections are undone when it ends;
// a continuation (stage 20) never contains one.

/// The rest of the computation: a persistent list of frames.
#[derive(Clone, Default)]
pub struct Chain(Option<Arc<Node>>);

pub struct Node {
    frame: Frame,
    next: Chain,
}

impl Chain {
    fn push(&mut self, frame: Frame) {
        let next = std::mem::take(self);
        self.0 = Some(Arc::new(Node { frame, next }));
    }
}

#[derive(Clone)]
enum Frame {
    /// The statements of a script from line, stmt. settle: Subst_settle first
    /// (the statement before has ended).
    Lines { sh: Ctx, script: Sexp, line: usize, stmt: usize, settle: bool },
    /// The clauses of an if from clause; test: the condition of clause has
    /// been run, look at $?.
    If { sh: Ctx, node: Sexp, clause: usize, test: bool },
    /// The next item of a for loop: values[index..], and the rest of the
    /// LazyList being read.
    For { sh: Ctx, name: Arc<str>, values: Arc<Vec<Value>>, index: usize, cur: Option<LazyList>, body: Sexp, id: u64 },
    /// The elements of a command list (list Element ...) from index.
    List { sh: Ctx, node: Sexp, index: usize },
    /// The end of a function call: return with this id ends here.
    Call { id: u64 },
    /// The end of the body of a reset: go on where the innermost reset (or
    /// call of a continuation) of the thread goes on.
    ResetEnd { sh: Ctx },
}

/// Run a chain to its end.
pub fn run(mut chain: Chain) -> R<()> {
    while let Some(node) = chain.0.take() {
        chain = node.next.clone();
        if let Err(flow) = step(&node.frame, &mut chain) {
            chain = unwind(chain, flow)?;
        }
    }
    Ok(())
}

/// Leave the frames up to the one that return, break or continue goes to;
/// any other Err ends the run.
fn unwind(mut chain: Chain, flow: Flow) -> R<Chain> {
    loop {
        let node = match chain.0.take() {
            None => return Err(flow),
            Some(n) => n,
        };
        match (&node.frame, &flow) {
            (Frame::Call { id }, Flow::Return(r)) if id == r => return Ok(node.next.clone()),
            (Frame::For { id, sh, .. }, Flow::Break(b)) if id == b => {
                sh.set_status("0");
                return Ok(node.next.clone());
            }
            (Frame::For { id, .. }, Flow::Continue(c)) if id == c => return Ok(Chain(Some(node))),
            // Leaving the body of a reset (a full variable list stops the command).
            (Frame::ResetEnd { sh }, _) => {
                sh.meta.lock().unwrap().pop();
            }
            _ => {}
        }
        chain = node.next.clone();
    }
}

fn step(frame: &Frame, chain: &mut Chain) -> R<()> {
    match frame {
        Frame::Lines { sh, script, line, stmt, settle } => step_lines(sh, script, *line, *stmt, *settle, chain),
        Frame::If { sh, node, clause, test } => step_if(sh, node, *clause, *test, chain),
        Frame::For { sh, name, values, index, cur, body, id } => step_for(sh, name, values, *index, cur.clone(), body, *id, chain),
        Frame::List { sh, node, index } => step_list(sh, node, *index, chain),
        Frame::Call { .. } => Ok(()),
        Frame::ResetEnd { sh } => {
            let info = sh.meta.lock().unwrap().pop().expect("the end of a reset without its frame");
            *chain = info.k;
            Ok(())
        }
    }
}

/// A script node holding the command lines (for the top level and $( )).
fn script_of(lines: &[Sexp]) -> Sexp {
    let mut v = Vec::with_capacity(lines.len() + 1);
    v.push(Sexp::Sym(Arc::from("script")));
    v.extend(lines.iter().cloned());
    Sexp::List(Arc::new(v))
}

pub fn eval_lines(sh: &Ctx, lines: &[Sexp]) -> R<()> {
    run_script(sh, &script_of(lines))
}

/// Run the command lines of a script node.
fn run_script(sh: &Ctx, script: &Sexp) -> R<()> {
    let mut chain = Chain::default();
    chain.push(Frame::Lines { sh: sh.clone(), script: script.clone(), line: 0, stmt: 0, settle: false });
    run(chain)
}

/// Run one statement to its end (the element of a pipeline, a job).
fn run_stmt(sh: &Ctx, stmt: &Sexp) -> R<()> {
    let mut chain = Chain::default();
    exec_stmt(sh, stmt, &mut chain)?;
    run(chain)
}

/// Push the frames that run a script; an empty one sets $? to 0 (if and for).
fn push_script(sh: &Ctx, script: &Sexp, chain: &mut Chain) {
    if script.children().is_empty() {
        sh.set_status("0");
        return;
    }
    chain.push(Frame::Lines { sh: sh.clone(), script: script.clone(), line: 0, stmt: 0, settle: false });
}

fn step_lines(sh: &Ctx, script: &Sexp, mut line: usize, mut stmt: usize, settle: bool, chain: &mut Chain) -> R<()> {
    if settle {
        subst_settle(sh);
    }
    let lines = script.children();
    let s = loop {
        let l = match lines.get(line) {
            None => return Ok(()),
            Some(l) => l,
        };
        match l.children().get(stmt) {
            Some(s) => break s,
            None => {
                line += 1;
                stmt = 0;
            }
        }
    };
    os::jitter();
    if sh.killed.load(Ordering::SeqCst) {
        return Err(Flow::Killed);
    }
    chain.push(Frame::Lines { sh: sh.clone(), script: script.clone(), line, stmt: stmt + 1, settle: true });
    exec_stmt(sh, s, chain)
}

/// Run a statement, or push the frames that run it.
fn exec_stmt(sh: &Ctx, stmt: &Sexp, chain: &mut Chain) -> R<()> {
    let pos = stmt.pos();
    let children = stmt.children();
    match stmt.tag() {
        "cmd" => exec_cmd(sh, pos, children, chain),
        "assign" => {
            for def in children {
                let c = def.children();
                let value = expand::def_value(sh, &c[1..])?;
                sh.set_var(c[0].str(), value);
            }
            sh.set_status("0");
            Ok(())
        }
        "if" => exec_if(sh, pos, stmt, chain),
        "for" => exec_for(sh, pos, stmt, chain),
        "break" | "continue" => eval_jump(sh, pos, stmt.tag(), children),
        "export" => {
            eval_export(sh, children)?;
            sh.set_status("0");
            Ok(())
        }
        "function" => {
            func_define(sh, children);
            sh.set_status("0");
            Ok(())
        }
        "return" => eval_return(sh, pos, children),
        "pipe" => eval_pipe(sh, children),
        "reset" => exec_reset(sh, pos, children, chain),
        "shift" => exec_shift(sh, pos, children, chain),
        _ => {
            chain.push(Frame::List { sh: sh.clone(), node: stmt.clone(), index: 0 });
            Ok(())
        }
    }
}

/// A command list: the jobs start at once, the other elements run in turn.
fn step_list(sh: &Ctx, node: &Sexp, index: usize, chain: &mut Chain) -> R<()> {
    let elems = node.children();
    for (i, elem) in elems.iter().enumerate().skip(index) {
        if elem.tag() == "bg" {
            eval_bg(sh, &elem.children()[0])?;
        } else {
            chain.push(Frame::List { sh: sh.clone(), node: node.clone(), index: i + 1 });
            return exec_stmt(sh, elem, chain);
        }
    }
    Ok(())
}

/// export item ...: set environment variables; local variables do not change.
fn eval_export(sh: &Ctx, items: &[Sexp]) -> R<()> {
    for item in items {
        let pos = item.pos();
        let c = item.children();
        let name = c[0].str();
        let value = if item.tag() == "name" {
            let v = sh.get_var(name);
            match v {
                None => return Err(sh.error(pos, &format!("export of an undefined local variable: {}", name))),
                Some(v) => expand::value_to_string(sh, pos, &v)?,
            }
        } else {
            env_value(sh, pos, &c[1..])?
        };
        sh.env.export(name, &value);
    }
    Ok(())
}

/// The value of an environment variable: "" without a right side; a LazyList is made one String.
fn env_value(sh: &Ctx, pos: Pos, values: &[Sexp]) -> R<String> {
    let v = expand::def_value(sh, values)?;
    expand::value_to_string(sh, pos, &v)
}

/// return word: a number from 0 to 255; the rest of the function is not run.
fn eval_return(sh: &Ctx, pos: Pos, children: &[Sexp]) -> R<()> {
    if let Some(msg) = &sh.no_return {
        return Err(sh.error(pos, msg));
    }
    let ret = match sh.ret {
        None => return Err(sh.error(pos, "return outside a function")),
        Some(r) => r,
    };
    let fields = expand::fields(sh, &children[0])?;
    let value = match fields.as_slice() {
        [Value::Str(w)] if is_digits(w) => w.parse::<u64>().ok().filter(|n| *n <= 255),
        _ => None,
    };
    match value {
        None => Err(sh.error(pos, "return needs a number from 0 to 255")),
        Some(n) => {
            sh.set_status(&n.to_string());
            Err(Flow::Return(ret))
        }
    }
}

// ── Functions ──

/// function name params { script }: in the functions of this scope, with a
/// copy of the local variables.
fn func_define(sh: &Ctx, children: &[Sexp]) {
    let name = children[0].str().to_string();
    let params = children[1].children();
    let names = params[0].list().iter().map(|n| n.str().to_string()).collect();
    let rest = params.get(1).map(|r| r.str().to_string()).unwrap_or_default();
    let f = Func {
        names,
        rest,
        body: children[2].clone(),
        scope: sh.scope.clone(),
        vars: sh.vars.lock().unwrap().clone(),
    };
    sh.scope.funcs.lock().unwrap().insert(name, Arc::new(f));
}

/// The function name, in the nearest scope going lexically outward.
fn func_lookup(sh: &Ctx, name: &str) -> Option<Arc<Func>> {
    let mut s = Some(sh.scope.clone());
    while let Some(scope) = s {
        if let Some(f) = scope.funcs.lock().unwrap().get(name) {
            return Some(f.clone());
        }
        s = scope.parent.clone();
    }
    None
}

/// Call a function: the arguments go to the parameters ("" for those
/// missing; the rest as a LazyList), and the body runs in a new scope.
fn func_push(sh: &Ctx, f: &Arc<Func>, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target, chain: &mut Chain) {
    let mut vars = f.vars.clone();
    let mut it = args.into_iter();
    for n in &f.names {
        vars.insert(n.clone(), Var::V(it.next().unwrap_or_else(|| Value::str(""))));
    }
    if !f.rest.is_empty() {
        vars.insert(f.rest.clone(), Var::V(Value::List(LazyList::from_vec(it.collect()))));
    }
    let id = new_id();
    let callee = Ctx {
        vars: Arc::new(Mutex::new(vars)),
        scope: Arc::new(Scope { funcs: Mutex::new(HashMap::new()), parent: Some(f.scope.clone()) }),
        in_function: true,
        ret: Some(id),
        loops: None,
        no_return: None,
        input,
        out,
        err,
        ..sh.clone()
    };
    sh.set_status("0");
    chain.push(Frame::Call { id });
    chain.push(Frame::Lines { sh: callee, script: f.body.clone(), line: 0, stmt: 0, settle: false });
}

// ── Function values and continuations (stage 20) ──
// A function value is called like a builtin: with the context, the
// position, the arguments, the standard input and outputs. It is a function
// made by f=function (...) { ... }, a bind, or a continuation of a shift.

pub enum FuncValue {
    Func(Arc<Func>),
    /// bind name|function value args ...: the scope of the bind finds the name.
    Bind { scope: Arc<Scope>, target: Value, args: Vec<Value> },
    Cont(Cont),
}

/// A continuation: the rest of the computation after a shift (up to the end
/// of its reset), the thread that made it, and the local variables and the
/// variable list of the shift that the values given to it go to.
pub struct Cont {
    resume: Chain,
    thread: std::thread::ThreadId,
    vars: Vars,
    params: Option<Sexp>,
    meta: Meta,
}

/// The function value of (fun params script): the local variables when it is
/// made, and the functions of the scope where it is made.
pub fn func_value(sh: &Ctx, children: &[Sexp]) -> Value {
    Value::Func(Arc::new(FuncValue::Func(Arc::new(func_make(sh, &children[0], &children[1])))))
}

fn func_make(sh: &Ctx, params: &Sexp, body: &Sexp) -> Func {
    let p = params.children();
    let names = p.first().map(|n| n.list().iter().map(|n| n.str().to_string()).collect()).unwrap_or_default();
    let rest = p.get(1).map(|r| r.str().to_string()).unwrap_or_default();
    Func { names, rest, body: body.clone(), scope: sh.scope.clone(), vars: sh.vars.lock().unwrap().clone() }
}

/// Call a function value, going on in chain.
#[allow(clippy::too_many_arguments)]
fn call_value(sh: &Ctx, pos: Pos, f: &Arc<FuncValue>, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target, chain: &mut Chain) -> R<()> {
    match &**f {
        FuncValue::Func(func) => {
            let (out, err) = body_targets(sh, pos, &out, &err);
            func_push(sh, func, args, input, out, err, chain);
            Ok(())
        }
        FuncValue::Bind { scope, target, args: bound } => {
            let mut all = bound.clone();
            all.extend(args);
            bind_call(sh, scope, pos, target, all, input, out, err, chain)
        }
        FuncValue::Cont(c) => cont_call(sh, pos, c, args, input, out, err, chain),
    }
}

/// Call a function value and wait for it (a command with redirections).
#[allow(clippy::too_many_arguments)]
fn call_value_direct(sh: &Ctx, pos: Pos, f: &Arc<FuncValue>, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target) -> R<()> {
    let mut chain = Chain::default();
    call_value(sh, pos, f, args, input, out, err, &mut chain)?;
    run(chain)
}

/// bind name|function value args ...: a function value that calls the name
/// or the function value with args before its own arguments. The name is
/// found when it is called, from the scope of the bind, as a command name.
fn builtin_bind(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, out: Target, _err: Target) -> R<()> {
    let msg = "bind: a command name or a function value is needed";
    let mut it = args.into_iter();
    let target = match it.next() {
        Some(v @ (Value::Str(_) | Value::Func(_))) => v,
        _ => return Err(sh.error(pos, msg)),
    };
    let f = Value::Func(Arc::new(FuncValue::Bind { scope: sh.scope.clone(), target, args: it.collect() }));
    target_write(sh, pos, &out, &f)?;
    sh.set_status("0");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bind_call(sh: &Ctx, scope: &Arc<Scope>, pos: Pos, target: &Value, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target, chain: &mut Chain) -> R<()> {
    let name = match target {
        Value::Func(f) => return call_value(sh, pos, f, args, input, out, err, chain),
        Value::Str(s) => s.to_string(),
        Value::List(_) => unreachable!(),
    };
    let inner = Ctx { scope: scope.clone(), ..sh.clone() };
    let (func, builtin, is_external) = cmd_resolve(&inner, Some(&name));
    if is_external {
        let max = arg_max(&inner, pos)?;
        let leaves = expand::values_leaves_in(&inner, &args, &mut Limit::new(max, pos))?;
        return external_run(&inner, pos, &name, leaves, &input, &out, &err);
    }
    if let Some(f) = &func {
        if name != "builtin" && name != "command" {
            let (out, err) = body_targets(&inner, pos, &out, &err);
            func_push(&inner, f, args, input, out, err, chain);
            return Ok(());
        }
    }
    cmd_run_resolved(&inner, pos, &name, args, input, out, err, func, builtin)
}

/// (reset script [redirs]): run the script; a shift in it takes the rest of
/// it as a continuation. return, break and continue cannot leave it.
fn exec_reset(sh: &Ctx, pos: Pos, children: &[Sexp], chain: &mut Chain) -> R<()> {
    let script = &children[0];
    let redirs = children.get(1).map(|r| r.children()).unwrap_or(&[]);
    if redirs.is_empty() {
        reset_push(sh, script, chain);
        return Ok(());
    }
    eval_redirected(sh, pos, redirs, |inner| {
        let mut chain = Chain::default();
        reset_push(inner, script, &mut chain);
        run(chain)
    })
}

fn reset_push(sh: &Ctx, script: &Sexp, chain: &mut Chain) {
    let body = Ctx { redirected: false, loops: None, no_return: Some(Arc::from("return in reset")), ..sh.clone() };
    sh.meta.lock().unwrap().push(ResetInfo { k: chain.clone(), sh: sh.clone(), input: sh.input.clone(), out: sh.out.clone(), err: sh.err.clone() });
    chain.push(Frame::ResetEnd { sh: body.clone() });
    chain.push(Frame::Lines { sh: body, script: script.clone(), line: 0, stmt: 0, settle: false });
}

/// (shift "k" [params] script): the rest of the computation up to the end of
/// the innermost reset becomes a function value in k; the rest of the reset
/// is dropped and the script runs in its place, like a function value made
/// here and called there.
fn exec_shift(sh: &Ctx, pos: Pos, children: &[Sexp], chain: &mut Chain) -> R<()> {
    let name = children[0].str();
    let (params, script) = if children.len() == 3 { (Some(children[1].clone()), &children[2]) } else { (None, &children[1]) };
    let info = {
        let mut meta = sh.meta.lock().unwrap();
        if meta.is_empty() {
            drop(meta);
            return Err(sh.error(pos, "shift outside reset"));
        }
        if sh.redirected {
            drop(meta);
            return Err(sh.error(pos, "shift: a redirection between reset and shift"));
        }
        meta.pop().unwrap()
    };
    let k = Value::Func(Arc::new(FuncValue::Cont(Cont {
        resume: chain.clone(),
        thread: std::thread::current().id(),
        vars: sh.vars.clone(),
        params,
        meta: sh.meta.clone(),
    })));
    let mut vars = sh.vars.lock().unwrap().clone();
    vars.insert(name.to_string(), Var::V(k));
    let body = Arc::new(Func { names: Vec::new(), rest: String::new(), body: script.clone(), scope: sh.scope.clone(), vars });
    let (out, err) = body_targets(&info.sh, pos, &info.out, &info.err);
    let mut next = info.k.clone();
    func_push(&info.sh, &body, Vec::new(), info.input.clone(), out, err, &mut next);
    *chain = next;
    Ok(())
}

/// Call a continuation: the place of the call becomes the innermost reset,
/// the standard input and outputs in the continuation become those of the
/// call, the values go to the variable list of the shift, and the rest after
/// the shift goes on.
#[allow(clippy::too_many_arguments)]
fn cont_call(sh: &Ctx, pos: Pos, c: &Cont, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target, chain: &mut Chain) -> R<()> {
    if std::thread::current().id() != c.thread {
        return Err(sh.error(pos, "a continuation called in another VProc"));
    }
    let (out, err) = body_targets(sh, pos, &out, &err);
    c.meta.lock().unwrap().push(ResetInfo { k: chain.clone(), sh: sh.clone(), input: input.clone(), out: out.clone(), err: err.clone() });
    let resume = retarget(&c.resume, &input, &out, &err);
    if let Some(p) = &c.params {
        let pc = p.children();
        let names: Vec<String> = pc.first().map(|n| n.list().iter().map(|n| n.str().to_string()).collect()).unwrap_or_default();
        let rest = pc.get(1).map(|r| r.str()).unwrap_or("");
        let mut vars = c.vars.lock().unwrap();
        let mut it = args.into_iter();
        for n in &names {
            vars.insert(n.clone(), Var::V(it.next().unwrap_or_else(|| Value::str(""))));
        }
        if !rest.is_empty() {
            vars.insert(rest.to_string(), Var::V(Value::List(LazyList::from_vec(it.collect()))));
        }
    }
    sh.set_status("0");
    *chain = resume;
    Ok(())
}

/// The frames of a continuation up to the end of its reset, with the standard
/// input and outputs of their contexts replaced.
fn retarget(chain: &Chain, input: &Arc<Input>, out: &Target, err: &Target) -> Chain {
    let set = |sh: &Ctx| Ctx { input: input.clone(), out: out.clone(), err: err.clone(), ..sh.clone() };
    let mut frames = Vec::new();
    let mut cur = chain.0.clone();
    while let Some(node) = cur {
        let f = match &node.frame {
            Frame::Lines { sh, script, line, stmt, settle } => Frame::Lines { sh: set(sh), script: script.clone(), line: *line, stmt: *stmt, settle: *settle },
            Frame::If { sh, node, clause, test } => Frame::If { sh: set(sh), node: node.clone(), clause: *clause, test: *test },
            Frame::For { sh, name, values, index, cur, body, id } => {
                Frame::For { sh: set(sh), name: name.clone(), values: values.clone(), index: *index, cur: cur.clone(), body: body.clone(), id: *id }
            }
            Frame::List { sh, node, index } => Frame::List { sh: set(sh), node: node.clone(), index: *index },
            Frame::Call { id } => Frame::Call { id: *id },
            Frame::ResetEnd { sh } => {
                frames.push(Frame::ResetEnd { sh: set(sh) });
                break;
            }
        };
        frames.push(f);
        cur = node.next.0.clone();
    }
    let mut out_chain = Chain::default();
    for f in frames.into_iter().rev() {
        out_chain.push(f);
    }
    out_chain
}

/// Call a function and wait for it (a command with redirections).
fn func_call(sh: &Ctx, f: &Arc<Func>, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target) -> R<()> {
    let mut chain = Chain::default();
    func_push(sh, f, args, input, out, err, &mut chain);
    run(chain)
}

// ── Simple commands ──

/// Run a simple command (cmd [env] words [redirs]). The definitions before
/// the command are in the environment of the expansion of its words. The
/// first word is expanded first to find the command; the others are
/// expanded as words of an external command or, for functions and
/// builtins, by the rules of the other words.
fn exec_cmd(sh: &Ctx, pos: Pos, children: &[Sexp], chain: &mut Chain) -> R<()> {
    let (mut defs, mut words, mut redirs): (&[Sexp], &[Sexp], &[Sexp]) = (&[], &[], &[]);
    for c in children {
        match c.tag() {
            "env" => defs = c.children(),
            "words" => words = c.children(),
            _ => redirs = c.children(),
        }
    }
    let redirected;
    let sh = if redirs.is_empty() {
        sh
    } else {
        redirected = Ctx { redirected: true, ..sh.clone() };
        &redirected
    };
    let sh = &cmd_scope(sh, defs)?;
    let max = arg_max(sh, pos)?;
    let mut limit = Limit::new(max, pos);
    let head = expand::cmd_head(sh, words, &mut limit)?;
    let (name, more, rest) = match head {
        Some((n, m, r)) => (Some(n), m, r),
        None => (None, Vec::new(), words.len()),
    };
    // A function value as the command (stage 20) is run like a builtin.
    let fval = match &name {
        Some(Value::Func(f)) => Some(f.clone()),
        _ => None,
    };
    let (func, builtin, is_external) = if fval.is_some() { (None, None, false) } else { cmd_resolve(sh, name.as_ref().and_then(|n| n.as_str())) };
    let mut args = if is_external {
        let leaves = expand::values_leaves_in(sh, &more, &mut limit)?;
        leaves.into_iter().map(Value::string).collect()
    } else {
        more
    };
    for w in &words[rest..] {
        if is_external {
            args.extend(expand::fields_in(sh, w, &mut limit)?);
        } else {
            args.extend(expand::args(sh, w)?);
        }
    }
    let (input, out, err, group) = match eval_redirs(sh, redirs)? {
        None => {
            sh.set_status("1");
            return Ok(());
        }
        Some(r) => r,
    };
    let name = match name {
        None => {
            capture_assign(sh, &group);
            sh.set_status("0");
            return Ok(());
        }
        Some(n) => n,
    };
    // Without redirections, a function call goes on in this chain (so that a
    // continuation can contain it); with them, it runs in a nested one.
    if redirs.is_empty() {
        if let Some(f) = &fval {
            return call_value(sh, pos, f, args, input, out, err, chain);
        }
        if let Some(f) = &func {
            if name.as_str() != Some("builtin") && name.as_str() != Some("command") {
                let (out, err) = body_targets(sh, pos, &out, &err);
                func_push(sh, f, args, input, out, err, chain);
                return Ok(());
            }
        }
    }
    if let Some(f) = fval {
        return capture_run(sh, &group, |s| call_value_direct(s, pos, &f, args, input, out, err));
    }
    let name = name.as_str().unwrap_or("").to_string();
    capture_run(sh, &group, |s| cmd_run_resolved(s, pos, &name, args, input, out, err, func, builtin))
}

/// The context with the layer of the definitions before a command in its
/// environment. They do not see each other: all are expanded in sh.
fn cmd_scope(sh: &Ctx, defs: &[Sexp]) -> R<Ctx> {
    if defs.is_empty() {
        return Ok(sh.clone());
    }
    let mut layer = HashMap::new();
    for def in defs {
        let c = def.children();
        let v = env_value(sh, def.pos(), &c[1..])?;
        layer.insert(c[0].str().to_string(), v);
    }
    Ok(Ctx { env: sh.env.push(layer), ..sh.clone() })
}

type Builtin = fn(&Ctx, Pos, Vec<Value>, Arc<Input>, Target, Target) -> R<()>;

/// The function and the builtin of a command name, and whether it is an
/// external command. Without a name, it is treated as external.
fn cmd_resolve(sh: &Ctx, name: Option<&str>) -> (Option<Arc<Func>>, Option<Builtin>, bool) {
    let name = match name {
        None => return (None, None, true),
        Some(n) => n,
    };
    if name == "builtin" || name == "command" {
        return (None, None, false);
    }
    if let Some(f) = func_lookup(sh, name) {
        return (Some(f), None, false);
    }
    if let Some(b) = builtin_lookup(name) {
        return (None, Some(b), false);
    }
    (None, None, true)
}

#[allow(clippy::too_many_arguments)]
fn cmd_run_resolved(sh: &Ctx, pos: Pos, name: &str, args: Vec<Value>, input: Arc<Input>, out: Target, err: Target, func: Option<Arc<Func>>, builtin: Option<Builtin>) -> R<()> {
    if name == "builtin" {
        let mut it = args.into_iter();
        let name2 = match it.next() {
            None => return Err(sh.error(pos, "builtin: a command name is needed")),
            Some(v) => arg_string(sh, pos, "builtin", v)?,
        };
        return match builtin_lookup(&name2) {
            None => Err(sh.error(pos, &format!("builtin: not a builtin command: {}", name2))),
            Some(b) => b(sh, pos, it.collect(), input, out, err),
        };
    }
    if name == "command" {
        let max = arg_max(sh, pos)?;
        let mut limit = Limit::new(max, pos);
        let leaves = expand::values_leaves_in(sh, &args, &mut limit)?;
        let mut it = leaves.into_iter();
        let name2 = match it.next() {
            None => return Err(sh.error(pos, "command: a command name is needed")),
            Some(n) => n,
        };
        return external_run(sh, pos, &name2, it.collect(), &input, &out, &err);
    }
    if let Some(f) = func {
        let (out, err) = body_targets(sh, pos, &out, &err);
        return func_call(sh, &f, args, input, out, err);
    }
    if let Some(b) = builtin {
        return b(sh, pos, args, input, out, err);
    }
    if args.iter().any(|v| matches!(v, Value::Func(_))) {
        return Err(sh.error(pos, expand::FUNC_STRING));
    }
    let strings = args.iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();
    external_run(sh, pos, name, strings, &input, &out, &err)
}

/// A String argument of a builtin; a LazyList is an error.
fn arg_string(sh: &Ctx, pos: Pos, command: &str, v: Value) -> R<String> {
    match v {
        Value::Str(s) => Ok(s.to_string()),
        Value::Func(_) => Err(sh.error(pos, expand::FUNC_STRING)),
        Value::List(_) => Err(sh.error(pos, &format!("{}: a LazyList is not allowed as an argument", command))),
    }
}

// ── if and for ──

/// Run run in the context with the redirections of an if or a for statement
/// (sh itself without them). Like a function, the body gets the targets
/// for a body.
fn eval_redirected(sh: &Ctx, pos: Pos, redirs: &[Sexp], run: impl FnOnce(&Ctx) -> R<()>) -> R<()> {
    if redirs.is_empty() {
        return run(sh);
    }
    let (input, out, err, group) = match eval_redirs(sh, redirs)? {
        None => {
            sh.set_status("1");
            return Ok(());
        }
        Some(r) => r,
    };
    let (out, err) = body_targets(sh, pos, &out, &err);
    let inner = Ctx { input, out, err, bg_captures: None, redirected: true, ..sh.clone() };
    capture_run(&inner, &group, run)
}

/// if: run the conditions in turn and the body of the first one whose $?
/// is 0, or else the else part. Without it, $? is 0.
fn exec_if(sh: &Ctx, pos: Pos, node: &Sexp, chain: &mut Chain) -> R<()> {
    let redirs = node.children().iter().find(|c| c.tag() == "redirs").map(|r| r.children()).unwrap_or(&[]);
    if redirs.is_empty() {
        chain.push(Frame::If { sh: sh.clone(), node: node.clone(), clause: 0, test: false });
        return Ok(());
    }
    eval_redirected(sh, pos, redirs, |inner| {
        let mut chain = Chain::default();
        chain.push(Frame::If { sh: inner.clone(), node: node.clone(), clause: 0, test: false });
        run(chain)
    })
}

fn step_if(sh: &Ctx, node: &Sexp, mut clause: usize, test: bool, chain: &mut Chain) -> R<()> {
    let children = node.children();
    let clauses: Vec<&Sexp> = children.iter().filter(|c| c.tag() == "clause").collect();
    if test {
        if sh.status() == "0" {
            push_script(sh, &clauses[clause].children()[1], chain);
            return Ok(());
        }
        clause += 1;
    }
    if clause < clauses.len() {
        chain.push(Frame::If { sh: sh.clone(), node: node.clone(), clause, test: true });
        push_script(sh, &clauses[clause].children()[0], chain);
        return Ok(());
    }
    match children.iter().find(|c| c.tag() == "else") {
        None => sh.set_status("0"),
        Some(e) => push_script(sh, &e.children()[0], chain),
    }
    Ok(())
}

/// for name in words; do script; done. break leaves the loop and continue
/// the iteration. $? is 0 at the end.
fn exec_for(sh: &Ctx, pos: Pos, node: &Sexp, chain: &mut Chain) -> R<()> {
    let redirs = node.children().get(3).map(|r| r.children()).unwrap_or(&[]);
    if redirs.is_empty() {
        return for_start(sh, node, chain);
    }
    eval_redirected(sh, pos, redirs, |inner| {
        let mut chain = Chain::default();
        for_start(inner, node, &mut chain)?;
        run(chain)
    })
}

fn for_start(sh: &Ctx, node: &Sexp, chain: &mut Chain) -> R<()> {
    let children = node.children();
    let name: Arc<str> = Arc::from(children[0].str());
    let mut values = Vec::new();
    for w in children[1].children() {
        values.extend(expand::args(sh, w)?);
    }
    let id = new_id();
    let looped = Ctx { loops: Some(Arc::new(LoopNode { name: name.to_string(), id, next: sh.loops.clone() })), ..sh.clone() };
    chain.push(Frame::For { sh: looped, name, values: Arc::new(values), index: 0, cur: None, body: children[2].clone(), id });
    Ok(())
}

/// The next item: a String value, or the next element of a LazyList value.
#[allow(clippy::too_many_arguments)]
fn step_for(sh: &Ctx, name: &Arc<str>, values: &Arc<Vec<Value>>, mut index: usize, mut cur: Option<LazyList>, body: &Sexp, id: u64, chain: &mut Chain) -> R<()> {
    let item = loop {
        if let Some(l) = cur.take() {
            if let Some((item, rest)) = l.uncons()? {
                cur = Some(rest);
                break item;
            }
            continue;
        }
        match values.get(index) {
            None => {
                sh.set_status("0");
                return Ok(());
            }
            Some(Value::List(l)) => {
                cur = Some(l.clone());
                index += 1;
            }
            Some(v) => {
                index += 1;
                break v.clone();
            }
        }
    };
    chain.push(Frame::For { sh: sh.clone(), name: name.clone(), values: values.clone(), index, cur, body: body.clone(), id });
    sh.set_var(name, item);
    chain.push(Frame::Lines { sh: sh.clone(), script: body.clone(), line: 0, stmt: 0, settle: false });
    Ok(())
}

/// break [name] and continue [name]: the innermost loop of that variable, or
/// the innermost loop. $? is 0.
fn eval_jump(sh: &Ctx, pos: Pos, tag: &str, children: &[Sexp]) -> R<()> {
    if sh.loops.is_none() {
        return Err(sh.error(pos, &format!("{} outside a loop", tag)));
    }
    let name = children.first().map(|c| c.str()).unwrap_or("");
    let mut cur = sh.loops.clone();
    while let Some(l) = cur {
        if name.is_empty() || l.name == name {
            sh.set_status("0");
            return Err(if tag == "break" { Flow::Break(l.id) } else { Flow::Continue(l.id) });
        }
        cur = l.next.clone();
    }
    Err(sh.error(pos, &format!("{}: no loop for {}", tag, name)))
}

// ── Other threads ──
// A command substitution (and, later, each element of a pipeline) runs in
// another thread. Variable definitions, function definitions, export, cd
// and $? in it do not change the outside; return, and break and continue to
// an outer loop, are errors.

/// The context of a child thread: copies of the local variables and the
/// environment, a new scope for functions (outside it, sh's), and $? from sh.
fn sh_child(sh: &Ctx, input: Arc<Input>, out: Target, err: Target, no_return: &str) -> Ctx {
    let global = Arc::new(Global {
        status: Mutex::new(sh.status()),
        last_bg: Mutex::new(sh.global.last_bg.lock().unwrap().clone()),
        interactive: sh.global.interactive,
        substs: Mutex::new(Vec::new()),
        jobs: Mutex::new(Vec::new()),
    });
    Ctx {
        global,
        vars: Arc::new(Mutex::new(sh.vars.lock().unwrap().clone())),
        env: sh.env.copy(),
        scope: Arc::new(Scope { funcs: Mutex::new(HashMap::new()), parent: Some(sh.scope.clone()) }),
        input,
        out,
        err,
        ret: None,
        no_return: Some(Arc::from(no_return)),
        loops: None,
        cwd: Arc::new(Mutex::new(sh.cwd())),
        abort: None,
        killed: Arc::new(AtomicBool::new(false)),
        proc_ref: Arc::new(Mutex::new(None)),
        pipe_members: Arc::new(Mutex::new(Vec::new())),
        cancel_in: None,
        bg_captures: None,
        redirected: false,
        meta: Arc::new(Mutex::new(Vec::new())),
        ..sh.clone()
    }
}

/// The target for a child thread (the same id): a new writer of a channel,
/// or a copy of the fd of a Stream, which the child closes when it ends
/// (so that a job or a $( ) that outlives a statement can go on writing to
/// a file that the statement closes).
fn child_target(t: &Target) -> Target {
    let dup = |fd: &Arc<Fd>| fd.dup().map(|o| Fd::new(std::fs::File::from(o))).unwrap_or_else(|_| fd.clone());
    let kind = match &t.kind {
        TKind::Chan(w) => TKind::Chan(w.chan.writer()),
        TKind::Stream(fd) => TKind::Stream(dup(fd)),
        TKind::File(fd, c, p) => TKind::File(dup(fd), c.clone(), *p),
        TKind::Pipe(fd, c) => TKind::Pipe(dup(fd), c.clone()),
        k => k.clone(),
    };
    Target { id: t.id, kind }
}

/// The targets for a child: the same one if they go to the same place.
fn child_targets(out: &Target, err: &Target) -> (Target, Target) {
    let o = child_target(out);
    let e = if out.id == err.id { o.clone() } else { child_target(err) };
    (o, e)
}

/// When a thread ends (or is stopped): close its outputs and cancel the
/// channel of its input.
fn sh_release(child: &Ctx) {
    for t in [&child.out, &child.err] {
        match &t.kind {
            TKind::Chan(w) => w.close(),
            TKind::Stream(fd) | TKind::File(fd, _, _) | TKind::Pipe(fd, _) => fd.close(),
            _ => {}
        }
    }
    if let Some(c) = &child.cancel_in {
        c.cancel();
    }
}

/// A thread started for a $( ), an element of a pipeline or a job.
#[derive(Clone)]
pub struct Member {
    ctx: Ctx,
    done: Arc<(Mutex<bool>, std::sync::Condvar)>,
}

impl Member {
    fn finished(&self) -> bool {
        *self.done.0.lock().unwrap()
    }
    fn wait(&self) {
        let mut d = self.done.0.lock().unwrap();
        while !*d {
            d = self.done.1.wait(d).unwrap();
        }
    }
}

/// Start a thread that runs body in child. An abort (the reader of its
/// output has finished), exit or kill ends the thread. At its end, its $( )
/// are finished, its outputs closed and the fds in owned closed, and then
/// done is called.
fn sh_start(mut child: Ctx, owned: Vec<Arc<Fd>>, body: impl FnOnce(&Ctx) -> R<()> + Send + 'static, done: impl FnOnce() + Send + 'static) -> Member {
    let id = new_id();
    child.abort = Some(id);
    let flag = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let member = Member { ctx: child.clone(), done: flag.clone() };
    spawn(move || {
        os::jitter();
        let _ = body(&child);
        subst_finish(&child);
        sh_release(&child);
        for fd in &owned {
            fd.close();
        }
        drop(child);
        done();
        *flag.0.lock().unwrap() = true;
        flag.1.notify_all();
    });
    member
}

/// Stop a thread: kill its external command (SIGTERM), close its outputs,
/// and stop the pipeline it waits for and its $( ) (not its jobs).
fn vproc_stop(m: &Member) {
    m.ctx.killed.store(true, Ordering::SeqCst);
    if let Some(pid) = *m.ctx.proc_ref.lock().unwrap() {
        os::terminate(pid);
    }
    sh_release(&m.ctx);
    let members = std::mem::take(&mut *m.ctx.pipe_members.lock().unwrap());
    for p in &members {
        vproc_stop(p);
    }
    let substs = m.ctx.global.substs.lock().unwrap().clone();
    for rec in substs {
        match &rec.kind {
            SubstKind::Chan(c) => c.cancel(),
            SubstKind::Os(st) => st.cancelled.store(true, Ordering::SeqCst),
        }
        vproc_stop(&rec.member);
    }
}

// ── Pipelines ──

/// Run the elements of a pipeline in threads, the standard output of each
/// being the standard input of the next. Wait for all of them; $? is that
/// of the last.
fn eval_pipe(sh: &Ctx, elems: &[Sexp]) -> R<()> {
    pipe_check(sh, elems)?;
    let members = pipe_start(sh, elems)?;
    *sh.pipe_members.lock().unwrap() = members.clone();
    for m in &members {
        m.wait();
    }
    sh.pipe_members.lock().unwrap().clear();
    let last = members.last().unwrap();
    sh.set_status(&last.ctx.status());
    Ok(())
}

/// Start the elements in turn. If one of two neighbours is one external
/// command (pipe_is_external), they are joined by an OS pipe, otherwise by a
/// channel that passes the values as they are.
fn pipe_start(sh: &Ctx, elems: &[Sexp]) -> R<Vec<Member>> {
    let mut members = Vec::new();
    let mut input = sh.input.clone();
    let mut cancel_in: Option<Arc<Chan>> = None;
    let mut owned: Vec<Arc<Fd>> = Vec::new();
    for (i, elem) in elems.iter().enumerate() {
        if i + 1 == elems.len() {
            let (out, err) = child_targets(&sh.out, &sh.err);
            members.push(pipe_element(sh, elem, input, out, err, cancel_in, owned));
            break;
        }
        let writes = pipe_is_external(sh, elem);
        let reads = pipe_is_external(sh, &elems[i + 1]);
        let err = child_target(&sh.err);
        if writes || reads {
            let (r, w) = std::io::pipe().map_err(|e| sh.error(None, &format!("__OS_pipe__: {}", os::strerror(&e))))?;
            let reader = Fd::new(std::fs::File::from(std::os::fd::OwnedFd::from(r)));
            let writer = Fd::new(std::fs::File::from(std::os::fd::OwnedFd::from(w)));
            let out = if writes {
                Target { id: new_id(), kind: TKind::Stream(writer) }
            } else {
                Target { id: new_id(), kind: TKind::Pipe(writer, Arc::new(sh.clone())) }
            };
            members.push(pipe_element(sh, elem, input, out, err, cancel_in.take(), std::mem::take(&mut owned)));
            input = Input::from_stream(reader.clone());
            owned = vec![reader];
        } else {
            let chan = Chan::new(PIPE_CAP);
            let out = Target { id: new_id(), kind: TKind::Chan(chan.writer()) };
            members.push(pipe_element(sh, elem, input, out, err, cancel_in.take(), std::mem::take(&mut owned)));
            input = Input::from_list(sh, chan.list());
            cancel_in = Some(chan);
        }
    }
    Ok(members)
}

fn pipe_element(sh: &Ctx, elem: &Sexp, input: Arc<Input>, out: Target, err: Target, cancel_in: Option<Arc<Chan>>, owned: Vec<Arc<Fd>>) -> Member {
    let mut child = sh_child(sh, input, out, err, "return in a pipeline");
    child.cancel_in = cancel_in;
    let elem = elem.clone();
    sh_start(child, owned, move |c| {
        let r = run_stmt(c, &elem);
        subst_settle(c);
        r
    }, || {})
}

/// Where a pipe and redirections fight for the input or output: after the
/// redirections, the pipe of an element that is not the last must have a
/// writer, and an element that is not the first must not change its input
/// with <.
fn pipe_check(sh: &Ctx, elems: &[Sexp]) -> R<()> {
    for (i, elem) in elems.iter().enumerate() {
        let (mut out, mut err) = ("pipe", "err");
        for redir in stmt_redirs(elem) {
            let op = redir.children()[0].str();
            match op {
                "<" => {
                    if i > 0 {
                        return Err(sh.error(redir.pos(), "the pipe has no reader"));
                    }
                }
                "1>&2" => out = err,
                "2>&1" => err = out,
                "2>" => err = "file",
                _ => out = "file",
            }
        }
        if out != "pipe" && err != "pipe" && i + 1 < elems.len() {
            return Err(sh.error(elem.pos(), "the pipe has no writer"));
        }
    }
    Ok(())
}

/// The redirections of a simple statement (cmd, if, for).
fn stmt_redirs(elem: &Sexp) -> &[Sexp] {
    let children = elem.children();
    match elem.tag() {
        "cmd" | "if" => children.iter().find(|c| c.tag() == "redirs").map(|r| r.children()).unwrap_or(&[]),
        _ if children.len() == 4 => children[3].children(),
        _ => &[],
    }
}

// ── Jobs ──

pub struct Job {
    id: String,
    members: Vec<Member>,
    /// The context whose $? is the job's (the last element of a pipeline).
    last: Ctx,
    killed: AtomicBool,
    specs: Vec<Arc<BgSpec>>,
}

static NEXT_JOB: AtomicU64 = AtomicU64::new(2);

/// pipeline &: start it in other threads without waiting. $! is its id. The
/// variable lists of a simple command (f > (a b . rest) &) put values that
/// wait until they are made into a and b, and into rest a LazyList that
/// grows as elements come (Bg_captures).
fn eval_bg(sh: &Ctx, pipeline: &Sexp) -> R<()> {
    let sh = &Ctx { detached: sh.detached || sh.global.interactive, ..sh.clone() };
    let (members, last, specs) = if pipeline.tag() == "pipe" {
        let elems = pipeline.children();
        pipe_check(sh, elems)?;
        let members = pipe_start(sh, elems)?;
        let last = members.last().unwrap().ctx.clone();
        (members, last, Vec::new())
    } else {
        let (out, err) = child_targets(&sh.out, &sh.err);
        let mut child = sh_child(sh, sh.input.clone(), out, err, "return in a background job");
        let specs = bg_captures(sh, stmt_redirs(pipeline));
        child.bg_captures = Some(Arc::new(Mutex::new(specs.iter().cloned().collect())));
        let p = pipeline.clone();
        let m = sh_start(child, Vec::new(), move |c| {
            let r = run_stmt(c, &p);
            subst_settle(c);
            r
        }, || {});
        let last = m.ctx.clone();
        (vec![m], last, specs)
    };
    let id = NEXT_JOB.fetch_add(1, Ordering::Relaxed).to_string();
    *sh.global.last_bg.lock().unwrap() = id.clone();
    sh.global.jobs.lock().unwrap().insert(0, Arc::new(Job { id, members, last, killed: AtomicBool::new(false), specs }));
    sh.set_status("0");
    Ok(())
}

/// Wait for a job and return its status: 143 if it was killed.
fn job_wait(job: &Job) -> String {
    for m in &job.members {
        m.wait();
    }
    if job.killed.load(Ordering::SeqCst) {
        return "143".to_string();
    }
    job.last.status()
}

/// Wait for every job of this thread (at the end of the script, and wait).
fn jobs_wait_all(sh: &Ctx) {
    loop {
        let job = {
            let mut jobs = sh.global.jobs.lock().unwrap();
            if jobs.is_empty() {
                return;
            }
            jobs.remove(0)
        };
        job_wait(&job);
    }
}

/// Stop a job, unless it has finished (then wait gives its real status).
fn job_kill(job: &Job) {
    if job.members.iter().all(|m| m.finished()) {
        return;
    }
    job.killed.store(true, Ordering::SeqCst);
    for m in &job.members {
        vproc_stop(m);
    }
    for s in &job.specs {
        s.finish();
    }
}

fn job_find(sh: &Ctx, id: &str) -> Option<Arc<Job>> {
    sh.global.jobs.lock().unwrap().iter().find(|j| j.id == id).cloned()
}

/// The values for the variable lists of a job's simple command. For each
/// > or 2> to a variable list, the caller's variables get values that wait
/// until they are made, and rest a LazyList that grows.
fn bg_captures(sh: &Ctx, redirs: &[Sexp]) -> Vec<Arc<BgSpec>> {
    let mut specs = Vec::new();
    for redir in redirs {
        let c = redir.children();
        if c[0].str() == "<" || c.len() < 2 || c[1].tag() != "params" {
            continue;
        }
        let params = c[1].children();
        let mut promises = Vec::new();
        for n in params[0].list() {
            let p = Promise::new();
            sh.vars.lock().unwrap().insert(n.str().to_string(), Var::P(p.clone()));
            promises.push(p);
        }
        let rest_name = params.get(1).map(|r| r.str()).unwrap_or("");
        let rest = if rest_name.is_empty() {
            None
        } else {
            let g = Grow::new();
            sh.set_var(rest_name, Value::List(g.list()));
            Some(g)
        };
        specs.push(Arc::new(BgSpec { promises, rest }));
    }
    specs
}

/// The values that a variable list of a job gives to the caller.
pub struct BgSpec {
    promises: Vec<Arc<Promise>>,
    rest: Option<Arc<Grow>>,
}

impl BgSpec {
    /// The count-th element (from 1) received.
    fn add(&self, count: usize, item: Value) {
        match self.promises.get(count - 1) {
            Some(p) => p.resolve(item),
            None => {
                if let Some(g) = &self.rest {
                    g.append(item);
                }
            }
        }
    }
    /// When the statement ends (or the job is killed): "" for the values not
    /// made, and the end of rest.
    fn finish(&self) {
        for p in &self.promises {
            p.resolve(Value::str(""));
        }
        if let Some(g) = &self.rest {
            g.close(None);
        }
    }
}

/// A LazyList that grows as elements are appended, until it is closed or
/// continued by another LazyList.
pub struct Grow {
    m: Mutex<(std::collections::VecDeque<Value>, Option<Option<LazyList>>)>,
    changed: std::sync::Condvar,
}

impl Grow {
    fn new() -> Arc<Grow> {
        Arc::new(Grow { m: Mutex::new((std::collections::VecDeque::new(), None)), changed: std::sync::Condvar::new() })
    }
    fn append(&self, v: Value) {
        let mut g = self.m.lock().unwrap();
        if g.1.is_none() {
            g.0.push_back(v);
            self.changed.notify_all();
        }
    }
    /// End the list, or continue it with tail (a second time does nothing).
    fn close(&self, tail: Option<LazyList>) {
        let mut g = self.m.lock().unwrap();
        if g.1.is_none() {
            g.1 = Some(tail);
            self.changed.notify_all();
        }
    }
    fn list(self: &Arc<Self>) -> LazyList {
        let me = self.clone();
        LazyList::new(move || {
            let mut g = me.m.lock().unwrap();
            loop {
                if let Some(v) = g.0.pop_front() {
                    drop(g);
                    return Ok(Some((v, me.list())));
                }
                if let Some(end) = &g.1 {
                    return match end {
                        None => Ok(None),
                        Some(l) => l.uncons(),
                    };
                }
                g = me.changed.wait(g).unwrap();
            }
        })
    }
}

// ── Command substitution ──

/// A $( ) that has not finished: by a channel, or by an OS pipe (one
/// external command).
pub struct Subst {
    kind: SubstKind,
    member: Member,
}

enum SubstKind {
    Chan(Arc<Chan>),
    Os(Arc<OsSubst>),
}

struct OsSubst {
    value: LazyList,
    reader: Arc<Fd>,
    cancelled: Arc<AtomicBool>,
    done: Mutex<bool>,
    changed: std::sync::Condvar,
}

/// $( script ): run the script in another thread, and return the LazyList of
/// the elements of its standard output (the lines of external commands),
/// made as it is read.
pub fn eval_subst(sh: &Ctx, _pos: Pos, script: &Sexp) -> R<Value> {
    let lines: Vec<Sexp> = script.children().to_vec();
    let err = child_target(&sh.err);
    if subst_is_external(sh, &lines) {
        let (r, w) = std::io::pipe().map_err(|e| sh.error(None, &format!("__OS_pipe__: {}", os::strerror(&e))))?;
        let reader = Fd::new(std::fs::File::from(std::os::fd::OwnedFd::from(r)));
        let writer = Fd::new(std::fs::File::from(std::os::fd::OwnedFd::from(w)));
        let out = Target { id: new_id(), kind: TKind::Stream(writer) };
        let child = sh_child(sh, sh.input.clone(), out, err, "return in a command substitution");
        let cancelled = Arc::new(AtomicBool::new(false));
        let value = subst_lines(reader.clone(), cancelled.clone());
        let st = Arc::new(OsSubst { value: value.clone(), reader, cancelled, done: Mutex::new(false), changed: std::sync::Condvar::new() });
        let st2 = st.clone();
        let handle = sh_start(child, Vec::new(), move |c| eval_lines(c, &lines), move || {
            *st2.done.lock().unwrap() = true;
            st2.changed.notify_all();
        });
        push_subst(sh, SubstKind::Os(st), handle);
        return Ok(Value::List(value));
    }
    let chan = Chan::new(PIPE_CAP);
    let out = Target { id: new_id(), kind: TKind::Chan(chan.writer()) };
    let child = sh_child(sh, sh.input.clone(), out, err, "return in a command substitution");
    let c2 = chan.clone();
    let handle = sh_start(child, Vec::new(), move |c| eval_lines(c, &lines), move || c2.set_done());
    let value = chan.list();
    push_subst(sh, SubstKind::Chan(chan), handle);
    Ok(Value::List(value))
}

fn push_subst(sh: &Ctx, kind: SubstKind, member: Member) {
    let rec = Arc::new(Subst { kind, member });
    sh.global.substs.lock().unwrap().insert(0, rec);
}

/// Whether a $( ) is one simple statement that starts one external command.
fn subst_is_external(sh: &Ctx, lines: &[Sexp]) -> bool {
    match lines {
        [line] => match line.children() {
            [stmt] => pipe_is_external(sh, stmt),
            _ => false,
        },
        _ => false,
    }
}

/// Whether elem is a simple statement whose first word is only quoted or
/// bare text, the name of an external command. Bare text with a glob
/// character could expand to the name of a function, so it is not.
fn pipe_is_external(sh: &Ctx, elem: &Sexp) -> bool {
    if elem.tag() != "cmd" {
        return false;
    }
    let words = match elem.children().iter().find(|c| c.tag() == "words") {
        Some(w) => w.children(),
        None => return false,
    };
    let parts = match words.first() {
        Some(w) => w.children(),
        None => return false,
    };
    if parts.len() != 1 {
        return false;
    }
    let tag = parts[0].tag();
    if tag != "lit" && tag != "sq" {
        return false;
    }
    let name = parts[0].children().first().map(|c| c.str()).unwrap_or("");
    if tag == "lit" && crate::glob::has_meta(name) {
        return false;
    }
    let (f, b, ext) = cmd_resolve(sh, Some(name));
    ext && f.is_none() && b.is_none()
}

/// The lines of the output of a $( ) by an OS pipe, read as the LazyList is
/// read. At the end, the pipe is closed; after cancelled, nothing is read.
fn subst_lines(reader: Arc<Fd>, cancelled: Arc<AtomicBool>) -> LazyList {
    let r = Arc::new(Mutex::new(BufReader::new(FdReader(reader.clone()))));
    os_lines(r, reader, cancelled)
}

fn os_lines(r: Arc<Mutex<BufReader<FdReader>>>, fd: Arc<Fd>, cancelled: Arc<AtomicBool>) -> LazyList {
    LazyList::new(move || {
        if cancelled.load(Ordering::SeqCst) {
            return Ok(None);
        }
        let mut buf = Vec::new();
        let n = r.lock().unwrap().read_until(b'\n', &mut buf).unwrap_or(0);
        if n == 0 {
            fd.close();
            return Ok(None);
        }
        let line = chomp(&String::from_utf8_lossy(&buf)).to_string();
        Ok(Some((Value::string(line), os_lines(r, fd, cancelled))))
    })
}

/// At the end of a statement, wait for each $( ) started by this thread to
/// finish, or until its writer waits because its output has not been read.
fn subst_settle(sh: &Ctx) {
    // The list stays in place while waiting, so that kill can stop them.
    let list = sh.global.substs.lock().unwrap().clone();
    if list.is_empty() {
        return;
    }
    let mut done = Vec::new();
    for rec in &list {
        if settle_one(rec) == chan::State::Done {
            done.push(rec.clone());
        }
    }
    sh.global.substs.lock().unwrap().retain(|r| !done.iter().any(|d| Arc::ptr_eq(d, r)));
}

fn settle_one(rec: &Subst) -> chan::State {
    match &rec.kind {
        SubstKind::Chan(c) => c.settle(),
        SubstKind::Os(st) => {
            // If the command has finished, read the rest of its output (what the
            // OS pipe holds). Otherwise read up to PIPE_CAP elements from the start
            // of the value: if the output ends before, wait for the command.
            if *st.done.lock().unwrap() {
                let _ = st.value.to_vec();
                return chan::State::Done;
            }
            let mut cur = st.value.clone();
            for _ in 0..PIPE_CAP {
                match cur.uncons() {
                    Ok(Some((_, rest))) => cur = rest,
                    _ => {
                        let mut d = st.done.lock().unwrap();
                        while !*d {
                            d = st.changed.wait(d).unwrap();
                        }
                        return chan::State::Done;
                    }
                }
            }
            chan::State::Blocked
        }
    }
}

/// At the end of a script or a thread: tell the $( ) that their output will
/// not be read, and wait for them. One without output runs to its end; one
/// whose writer waits stops when it writes next.
fn subst_finish(sh: &Ctx) {
    let list = std::mem::take(&mut *sh.global.substs.lock().unwrap());
    for rec in list {
        match &rec.kind {
            SubstKind::Chan(c) => c.cancel(),
            SubstKind::Os(st) => {
                st.cancelled.store(true, Ordering::SeqCst);
                st.reader.close();
            }
        }
        rec.member.wait();
    }
}

// ── Builtins ──

fn builtin_lookup(name: &str) -> Option<Builtin> {
    if name == "bind" {
        return Some(builtin_bind);
    }
    Some(match name {
        "echo" => builtin_echo,
        "cd" => builtin_cd,
        "defined" => builtin_defined,
        "exit" => builtin_exit,
        "stdin" => builtin_stdin,
        "wait" => builtin_wait,
        "kill" => builtin_kill,
        _ => return None,
    })
}

/// wait [id ...]: without ids, wait for every job started by this thread
/// with & ($? 0). With ids, wait for each; $? is that of the last; 127 for
/// one that does not exist (or was waited for).
fn builtin_wait(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, _out: Target, _err: Target) -> R<()> {
    if args.is_empty() {
        jobs_wait_all(sh);
        sh.set_status("0");
        return Ok(());
    }
    for a in args {
        let id = arg_string(sh, pos, "wait", a)?;
        match job_find(sh, &id) {
            None => sh.set_status("127"),
            Some(job) => {
                sh.global.jobs.lock().unwrap().retain(|j| j.id != id);
                let st = job_wait(&job);
                sh.set_status(&st);
            }
        }
    }
    Ok(())
}

/// kill id ...: stop jobs started by this thread. A job that does not exist
/// gives a message and $? 1.
fn builtin_kill(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, _out: Target, _err: Target) -> R<()> {
    if args.is_empty() {
        return Err(sh.error(pos, "kill: a job ID is needed"));
    }
    sh.set_status("0");
    for a in args {
        let id = arg_string(sh, pos, "kill", a)?;
        match job_find(sh, &id) {
            None => {
                sh.warn(pos, &format!("kill: no such job: {}", id));
                sh.set_status("1");
            }
            Some(job) => job_kill(&job),
        }
    }
    Ok(())
}

/// stdin: write the elements of standard input to standard output, moving
/// its position past them. To a variable list (stdin > (a . rest)), only as
/// many as the variables, and rest gets the rest as it is.
fn builtin_stdin(sh: &Ctx, pos: Pos, args: Vec<Value>, input: Arc<Input>, out: Target, _err: Target) -> R<()> {
    if !args.is_empty() {
        return Err(sh.error(pos, "stdin: no arguments are allowed"));
    }
    input.touched.store(true, Ordering::SeqCst);
    if let TKind::Capture(r) = &out.kind {
        while r.items.lock().unwrap().len() < r.names.len() {
            let list = input.list.lock().unwrap().clone();
            match list.uncons()? {
                None => break,
                Some((item, rest)) => {
                    r.add(item);
                    *input.list.lock().unwrap() = rest;
                }
            }
        }
        if !r.rest.is_empty() && r.items.lock().unwrap().len() >= r.names.len() {
            let rest = input.list.lock().unwrap().clone();
            *r.direct.lock().unwrap() = Some(rest.clone());
            if let Some(g) = r.spec.as_ref().and_then(|s| s.rest.clone()) {
                g.close(Some(rest));
            }
        }
        sh.set_status("0");
        return Ok(());
    }
    loop {
        let list = input.list.lock().unwrap().clone();
        match list.uncons()? {
            None => break,
            Some((item, rest)) => {
                target_write(sh, pos, &out, &item)?;
                *input.list.lock().unwrap() = rest;
            }
        }
    }
    sh.set_status("0");
    Ok(())
}

/// exit [n]: end the shell with status n (or $?), a number from 0 to 255.
fn builtin_exit(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, _out: Target, _err: Target) -> R<()> {
    if args.len() > 1 {
        return Err(sh.error(pos, "exit: too many arguments"));
    }
    let status = match args.into_iter().next() {
        None => sh.status(),
        Some(v) => arg_string(sh, pos, "exit", v)?,
    };
    let code = match status.parse::<u64>() {
        Ok(n) if is_digits(&status) && n <= 255 => n,
        _ => return Err(sh.error(pos, "exit: a number from 0 to 255 is needed")),
    };
    sh.set_status(&code.to_string());
    Err(Flow::Exit(code as i32))
}

/// echo value ...: one element on standard output. One argument is written
/// as it is (also a LazyList); none is ""; several are made Strings and
/// joined with spaces.
fn builtin_echo(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, out: Target, _err: Target) -> R<()> {
    let line = match args.len() {
        0 => Value::str(""),
        1 => args.into_iter().next().unwrap(),
        _ => {
            let mut strs = Vec::with_capacity(args.len());
            for a in &args {
                strs.push(expand::value_to_string(sh, pos, a)?);
            }
            Value::string(strs.join(" "))
        }
    };
    target_write(sh, pos, &out, &line)?;
    sh.set_status("0");
    Ok(())
}

/// cd [dir]: change the current directory (HOME without an argument).
fn builtin_cd(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, _out: Target, _err: Target) -> R<()> {
    let dir = match args.into_iter().next() {
        None => match sh.env.lookup("HOME") {
            None => {
                sh.warn(pos, "cd: HOME is not set");
                sh.set_status("1");
                return Ok(());
            }
            Some(h) => h,
        },
        Some(v) => arg_string(sh, pos, "cd", v)?,
    };
    let target = if dir.is_empty() { PathBuf::new() } else { sh.cwd().join(&dir) };
    let r = std::fs::metadata(&target).and_then(|st| {
        if !st.is_dir() {
            Err(os::io_error(20))
        } else if !os::can_search(&target) {
            Err(os::io_error(13))
        } else {
            std::fs::canonicalize(&target)
        }
    });
    match r {
        Ok(real) => {
            *sh.cwd.lock().unwrap() = real;
            sh.set_status("0");
        }
        Err(e) => {
            sh.warn(pos, &format!("cd: {}: {}", os::strerror(&e), dir));
            sh.set_status("1");
        }
    }
    Ok(())
}

/// defined [-l] name: $? is 0 if the variable is defined, else 1. With -l,
/// only local variables count.
fn builtin_defined(sh: &Ctx, pos: Pos, args: Vec<Value>, _in: Arc<Input>, _out: Target, _err: Target) -> R<()> {
    let mut it = args.into_iter();
    let first = match it.next() {
        None => return Err(sh.error(pos, "defined: a variable name is needed")),
        Some(v) => arg_string(sh, pos, "defined", v)?,
    };
    let local = first == "-l";
    let mut names: Vec<Value> = if local { Vec::new() } else { vec![Value::string(first)] };
    names.extend(it);
    let mut names = names.into_iter();
    let name = match names.next() {
        None => return Err(sh.error(pos, "defined: a variable name is needed")),
        Some(n) => n,
    };
    if names.next().is_some() {
        return Err(sh.error(pos, "defined: too many arguments"));
    }
    let name = arg_string(sh, pos, "defined", name)?;
    let found = if local { sh.vars.lock().unwrap().contains_key(&name) } else { sh.defined(pos, &name)? };
    sh.set_status(if found { "0" } else { "1" });
    Ok(())
}

// ── External commands ──

/// The byte limit of the arguments of an external command: the environment
/// variable ARG_MAX (not a local variable), or the value of the OS.
fn arg_max(sh: &Ctx, pos: Pos) -> R<i64> {
    let v = match sh.env.lookup("ARG_MAX") {
        None => return Ok(os::arg_max()),
        Some(v) => v,
    };
    let msg = format!("ARG_MAX is not a non-negative integer: {}", v);
    match v.trim().parse::<f64>() {
        Ok(n) if n.is_finite() && n.fract() == 0.0 && n >= 0.0 => Ok(n as i64),
        _ => Err(sh.error(pos, &msg)),
    }
}

/// Start an external command, wait for it and set $?. Output that goes to
/// a channel or a variable list is read from a pipe line by line; standard
/// input made from a list is written to a pipe by another thread.
fn external_run(sh: &Ctx, pos: Pos, name: &str, args: Vec<String>, input: &Arc<Input>, out: &Target, err: &Target) -> R<()> {
    let max = arg_max(sh, pos)?;
    let bytes: i64 = std::iter::once(name).chain(args.iter().map(|s| s.as_str())).map(|s| s.len() as i64 + 1).sum();
    if bytes > max {
        return Err(sh.error(pos, &expand::over_message(max)));
    }
    let mut cmd = Command::new(name);
    cmd.args(&args).env_clear().envs(sh.env.dict()).current_dir(sh.cwd());
    // Standard output and error: the Stream itself, or a pipe to read.
    let mut pumps: Vec<(std::io::PipeReader, Target)> = Vec::new();
    let out_os = os_stream(out);
    let err_os = os_stream(err);
    let pipe = |sh: &Ctx| std::io::pipe().map_err(|e| sh.error(pos, &format!("__OS_pipe__: {}", os::strerror(&e))));
    match (out_os, err_os) {
        (Some(o), Some(e)) => {
            cmd.stdout(o).stderr(e);
        }
        (None, Some(e)) => {
            let (r, w) = pipe(sh)?;
            cmd.stdout(w).stderr(e);
            pumps.push((r, out.clone()));
        }
        (Some(o), None) => {
            let (r, w) = pipe(sh)?;
            cmd.stdout(o).stderr(w);
            pumps.push((r, err.clone()));
        }
        (None, None) if out.id == err.id => {
            let (r, w) = pipe(sh)?;
            let w2 = w.try_clone().map_err(|e| sh.error(pos, &os::strerror(&e)))?;
            cmd.stdout(w).stderr(w2);
            pumps.push((r, out.clone()));
        }
        (None, None) => {
            let (r, w) = pipe(sh)?;
            let (r2, w2) = pipe(sh)?;
            cmd.stdout(w).stderr(w2);
            pumps.push((r, out.clone()));
            pumps.push((r2, err.clone()));
        }
    }
    let feed = match external_input(input) {
        Some(fd) => {
            cmd.stdin(stdio_of(&fd));
            None
        }
        None => {
            let (r, w) = pipe(sh)?;
            cmd.stdin(r);
            Some(w)
        }
    };
    if sh.detached {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    os::jitter();
    // Check killed and keep the pid under the same lock as vproc_stop, so that a
    // kill either finds the pid or stops the command before it starts.
    let mut pref = sh.proc_ref.lock().unwrap();
    if sh.killed.load(Ordering::SeqCst) {
        return Err(Flow::Killed);
    }
    let spawned = cmd.spawn();
    drop(cmd);
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => {
            drop(pref);
            let msg = os::strerror(&e);
            sh.warn(pos, &format!("{}: {}: {}", name, msg, name));
            let status = if msg.contains("Permission denied") || msg.contains("Argument list too long") { "126" } else { "127" };
            sh.set_status(status);
            return Ok(());
        }
    };
    *pref = Some(child.id());
    drop(pref);
    let stop = Arc::new(AtomicBool::new(false));
    if let Some(w) = feed {
        let input = input.clone();
        let stop = stop.clone();
        spawn(move || feed_loop(input, w, stop));
    }
    // The second pipe (standard error to another place) is read by another thread.
    let mut pumper = None;
    if pumps.len() == 2 {
        let (r2, t2) = pumps.pop().unwrap();
        let t2 = child_target(&t2);
        let psh = Ctx { abort: Some(new_id()), ..sh.clone() };
        pumper = Some(spawn(move || {
            let _ = target_pump(&psh, &t2, r2);
            if let TKind::Chan(w) = &t2.kind {
                w.close();
            }
        }));
    }
    let mut result = Ok(());
    if let Some((r, t)) = pumps.pop() {
        // If writing stops the command (the reader has finished, the variable list
        // is full), close the pipe first (the command stops with SIGPIPE) and wait.
        result = target_pump(sh, &t, r);
    }
    if let Some(p) = pumper {
        let _ = p.join();
    }
    let st = child.wait();
    *sh.proc_ref.lock().unwrap() = None;
    stop.store(true, Ordering::SeqCst);
    result?;
    use std::os::unix::process::ExitStatusExt;
    let status = match st {
        Ok(s) => match (s.code(), s.signal()) {
            (Some(c), _) => c.to_string(),
            (None, Some(sig)) => (128 + sig).to_string(),
            _ => "1".to_string(),
        },
        Err(_) => "1".to_string(),
    };
    sh.set_status(&status);
    Ok(())
}

/// Read the lines of a pipe (without their newlines) and write them to the target.
fn target_pump(sh: &Ctx, t: &Target, r: std::io::PipeReader) -> R<()> {
    let mut reader = BufReader::new(r);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => return Ok(()),
            Ok(_) => {}
        }
        let line = chomp(&String::from_utf8_lossy(&buf)).to_string();
        target_write(sh, None, t, &Value::string(line))?;
    }
}

/// Write the elements of standard input, each made a String with a newline,
/// to the pipe of a command, moving the position of the input past each
/// element written. Stop at the end, when the command has finished, or when
/// the pipe cannot be written (the command closed it).
fn feed_loop(input: Arc<Input>, mut w: std::io::PipeWriter, stop: Arc<AtomicBool>) {
    use std::io::Write;
    loop {
        let list = input.list.lock().unwrap().clone();
        let (item, rest) = match list.uncons() {
            Ok(Some(x)) => x,
            _ => return,
        };
        if stop.load(Ordering::SeqCst) {
            return;
        }
        let line = match (&item, &input.sh) {
            (Value::Str(s), _) => s.to_string(),
            (v, Some(sh)) => match line_to_string(sh, None, v) {
                Ok(s) => s,
                Err(_) => return,
            },
            (_, None) => return,
        };
        if w.write_all(format!("{}\n", line).as_bytes()).is_err() {
            return;
        }
        *input.list.lock().unwrap() = rest;
    }
}

fn stdio_of(fd: &Arc<Fd>) -> Stdio {
    match fd.dup() {
        Ok(o) => Stdio::from(o),
        Err(_) => Stdio::null(),
    }
}

/// The Stream of a target that an external command can write to as it is,
/// or None for a channel or a variable list.
fn os_stream(t: &Target) -> Option<Stdio> {
    match &t.kind {
        TKind::Top(_) => Some(Stdio::inherit()),
        TKind::Stream(fd) | TKind::File(fd, _, _) | TKind::Pipe(fd, _) => Some(stdio_of(fd)),
        TKind::Chan(_) | TKind::Capture(_) => None,
    }
}

/// The standard input for an external command: the Stream itself while the
/// input has not been read as a LazyList; otherwise None (a pipe is fed).
fn external_input(input: &Arc<Input>) -> Option<Arc<Fd>> {
    match &input.stream {
        Some(fd) if !input.touched.load(Ordering::SeqCst) => Some(fd.clone()),
        _ => None,
    }
}

// ── Redirections ──

/// Apply the redirections from left to right: the standard input, output
/// and error, or None if a file cannot be opened (with a message).
type Redirected = (Arc<Input>, Target, Target, Option<Arc<Group>>);

fn eval_redirs(sh: &Ctx, redirs: &[Sexp]) -> R<Option<Redirected>> {
    let mut group: Option<Arc<Group>> = None;
    let mut used = [false; 3];
    let mut input = sh.input.clone();
    let mut out = sh.out.clone();
    let mut err = sh.err.clone();
    for redir in redirs {
        let pos = redir.pos();
        let c = redir.children();
        let op = c[0].str();
        let fd = match op {
            "<" => 0,
            "2>" | "2>&1" => 2,
            _ => 1,
        };
        if used[fd] {
            return Err(sh.error(pos, "duplicate redirection"));
        }
        used[fd] = true;
        match op {
            "1>&2" => {
                out = err.clone();
                continue;
            }
            "2>&1" => {
                err = out.clone();
                continue;
            }
            _ => {}
        }
        let target = &c[1];
        if target.tag() == "params" {
            if fd == 0 {
                input = input_vars(sh, pos, target.children())?;
                continue;
            }
            let g = group.get_or_insert_with(|| Arc::new(Group { records: Mutex::new(Vec::new()), has_rest: AtomicBool::new(false), stopped: AtomicBool::new(false) }));
            let params = target.children();
            let names: Vec<String> = params[0].list().iter().map(|n| n.str().to_string()).collect();
            let rest = params.get(1).map(|r| r.str().to_string()).unwrap_or_default();
            if !rest.is_empty() {
                g.has_rest.store(true, Ordering::SeqCst);
            }
            let spec = sh.bg_captures.as_ref().and_then(|q| q.lock().unwrap().pop_front());
            let record = Arc::new(Record { names, rest, items: Mutex::new(Vec::new()), direct: Mutex::new(None), group: Arc::downgrade(g), spec });
            g.records.lock().unwrap().push(record.clone());
            let t = Target { id: new_id(), kind: TKind::Capture(record) };
            if fd == 1 {
                out = t;
            } else {
                err = t;
            }
            continue;
        }
        let fields = expand::fields(sh, target)?;
        if fields.len() != 1 {
            return Err(sh.error(pos, "ambiguous redirect"));
        }
        let path = match &fields[0] {
            Value::Str(p) => p.to_string(),
            _ => return Err(sh.error(pos, expand::FUNC_STRING)),
        };
        let path = &path;
        let full = sh.cwd().join(path);
        let mut opts = std::fs::OpenOptions::new();
        match op {
            "<" => opts.read(true),
            ">>" => opts.append(true).create(true),
            _ => opts.write(true).create(true).truncate(true),
        };
        let file = match opts.open(if path.is_empty() { PathBuf::new() } else { full }) {
            Ok(f) => f,
            Err(e) => {
                sh.warn(pos, &format!("{}: {}: {}", path, os::strerror(&e), path));
                return Ok(None);
            }
        };
        let f = Fd::new(file);
        if fd == 0 {
            input = Input::from_stream(f);
        } else {
            let t = Target { id: new_id(), kind: TKind::Stream(f) };
            if fd == 1 {
                out = t;
            } else {
                err = t;
            }
        }
    }
    Ok(Some((input, out, err, group)))
}

/// < (a b . rest): the values of a and b in turn, then the elements of the
/// value of rest (a String is one element).
fn input_vars(sh: &Ctx, pos: Pos, params: &[Sexp]) -> R<Arc<Input>> {
    let mut items = Vec::new();
    for n in params[0].list() {
        items.push(sh.lookup(pos, n.str())?);
    }
    let rest = params.get(1).map(|r| r.str()).unwrap_or("");
    let mut tail = LazyList::empty();
    if !rest.is_empty() {
        tail = match sh.lookup(pos, rest)? {
            Value::List(l) => l,
            v => LazyList::cons(v, LazyList::empty()),
        };
    }
    let mut list = tail;
    for v in items.into_iter().rev() {
        list = LazyList::cons(v, list);
    }
    Ok(Input::from_list(sh, list))
}

// ── Redirections to variable lists ──
// > (a b . rest) and 2> (a b . rest): the elements received go to a and b
// in turn, and the others to rest. The variable lists of one statement are a
// group. When every list without rest has as many elements as variables and
// no list has rest, the next write stops the command (as SIGPIPE does; $?
// is 0). Until then, elements beyond the variables are dropped.

pub struct Group {
    records: Mutex<Vec<Arc<Record>>>,
    has_rest: AtomicBool,
    stopped: AtomicBool,
}

pub struct Record {
    names: Vec<String>,
    rest: String,
    items: Mutex<Vec<Value>>,
    /// stdin > (a . rest): the rest of standard input, given to rest as it is.
    direct: Mutex<Option<LazyList>>,
    group: std::sync::Weak<Group>,
    /// The values given to the caller of a job (Bg_captures).
    spec: Option<Arc<BgSpec>>,
}

impl Record {
    /// Add a received element.
    fn add(&self, item: Value) {
        let mut items = self.items.lock().unwrap();
        items.push(item.clone());
        if let Some(s) = &self.spec {
            s.add(items.len(), item);
        }
    }
}

fn capture_put(sh: &Ctx, r: &Arc<Record>, item: Value) -> R<()> {
    let keep = r.items.lock().unwrap().len() < r.names.len() || !r.rest.is_empty();
    if keep {
        r.add(item);
        return Ok(());
    }
    let g = match r.group.upgrade() {
        Some(g) => g,
        None => return Ok(()),
    };
    if g.has_rest.load(Ordering::SeqCst) {
        return Ok(());
    }
    let satisfied = g.records.lock().unwrap().iter().all(|r| r.items.lock().unwrap().len() >= r.names.len());
    if !satisfied {
        return Ok(());
    }
    g.stopped.store(true, Ordering::SeqCst);
    Err(sh_abort(sh))
}

/// Run body in a context in which the command can be stopped, then assign
/// the variables. A stopped command has $? 0.
fn capture_run(sh: &Ctx, group: &Option<Arc<Group>>, body: impl FnOnce(&Ctx) -> R<()>) -> R<()> {
    let g = match group {
        None => return body(sh),
        Some(g) => g,
    };
    let id = new_id();
    let inner = Ctx { abort: Some(id), bg_captures: None, ..sh.clone() };
    let r = match body(&inner) {
        Err(Flow::Abort(a)) if a == id => Ok(()),
        other => other,
    };
    if matches!(r, Err(Flow::Exit(_)) | Err(Flow::Error)) {
        return r;
    }
    if g.stopped.load(Ordering::SeqCst) {
        sh.set_status("0");
    }
    capture_assign(sh, group);
    r
}

/// Assign the values of the group to the local variables of sh: "" for the
/// variables without one, and the LazyList of the others to rest.
fn capture_assign(sh: &Ctx, group: &Option<Arc<Group>>) {
    let g = match group {
        None => return,
        Some(g) => g,
    };
    let records = g.records.lock().unwrap().clone();
    let mut vars = sh.vars.lock().unwrap();
    for r in records {
        if let Some(s) = &r.spec {
            s.finish();
        }
        let items = r.items.lock().unwrap().clone();
        let mut it = items.into_iter();
        for n in &r.names {
            vars.insert(n.clone(), Var::V(it.next().unwrap_or_else(|| Value::str(""))));
        }
        if !r.rest.is_empty() {
            let mut list = r.direct.lock().unwrap().clone().unwrap_or_else(LazyList::empty);
            let more: Vec<Value> = it.collect();
            for v in more.into_iter().rev() {
                list = LazyList::cons(v, list);
            }
            vars.insert(r.rest.clone(), Var::V(Value::List(list)));
        }
    }
}
