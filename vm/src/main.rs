//! conma — run a ConMa VProc to completion, as test/vproc_run.py does.
//!
//! Usage:
//!     conma [-t|--trace] <module_file> <func_name>
//!     conma [-t|--trace] -m|--main <module_file> [<arg> ...]
//!     conma build [--warn-arity] [--include <dir>]... <source>
//!     conma run [--include <dir>]... <source> [<arg> ...]
//!
//! build makes the module of a ConMa source file (and the files it includes)
//! with the ConMa frontend that this executable embeds (vm/frontend.module.se,
//! include/frontend.se), and writes it to standard output, as bin/includer
//! and bin/resolver do; the standard library (CONMA_INCLUDE, or include/ of
//! the repository) is searched after the --include directories. run makes the
//! module and runs it with __start__ and the arguments, as --main does.
//! Neither needs Python.
//!
//! --trace prints the VProc state to standard error before the first step and
//! after each step. --main starts the program with __start__ and passes the
//! remaining arguments as a Seq of Strings.
//!
//! With CONMA_SEED=<n>, the scheduler chooses the VProc to run next at random
//! (see "Scheduling and Termination" in docs/DESIGN.md).

mod alloc;
mod gc;
mod module;
mod transpiled;
mod prim;
mod profile;
mod sched;
mod sexp;
mod step;
mod sys;
mod trace;
mod value;
mod vproc;

#[global_allocator]
static GLOBAL: alloc::Pool = alloc::Pool::new();

use std::process::exit;
use std::rc::Rc;
use value::{string_value, SeqClosure, Value};

fn usage_error(msg: &str) -> ! {
    eprintln!("usage: conma [-t] [-m] module_file [func_name | arg ...]");
    eprintln!("conma: error: {}", msg);
    exit(2);
}

/// The module of the ConMa frontend (include/frontend.se), made by
/// bin/update-frontend and kept in the repository.
const FRONTEND: &str = include_str!("../frontend.module.se");

/// conma build: run the embedded frontend with the options and the source,
/// the standard library last among the include directories (include/fast, then include/).
fn build(args: Vec<String>) -> i32 {
    let mut a = args;
    let source = match a.pop() {
        Some(s) if !s.starts_with("--") => s,
        _ => usage_error("build: a source file is needed"),
    };
    a.extend(prim::vprocs::include_args());
    a.push(source);
    run_text(FRONTEND.to_string(), "conma/vm/frontend.module.se", "__start__".into(), Some(a), false)
}

/// conma run: make the module with conma build (another process of this
/// executable, whose standard error is this one's) and run it.
fn run_source(args: Vec<String>) -> i32 {
    let mut opts = Vec::new();
    let mut it = args.into_iter();
    let source = loop {
        match it.next() {
            Some(o) if o == "--include" => {
                opts.push(o);
                match it.next() {
                    Some(d) => opts.push(d),
                    None => usage_error("run: --include needs a directory"),
                }
            }
            Some(s) => break s,
            None => usage_error("run: a source file is needed"),
        }
    };
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let out = std::process::Command::new(exe)
        .arg("build")
        .args(&opts)
        .arg(&source)
        .stderr(std::process::Stdio::inherit())
        .env_remove("CONMA_TRANSPILED_REPORT")
        .env_remove("CONMA_PROFILE")
        .env_remove("CONMA_TRANSPILED_COUNTS")
        .output();
    let out = match out {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    if !out.status.success() {
        return out.status.code().unwrap_or(1);
    }
    run_text(String::from_utf8_lossy(&out.stdout).into_owned(), &source, "__start__".into(), Some(it.collect()), false)
}

fn run(args: Vec<String>) -> i32 {
    match args.first().map(|s| s.as_str()) {
        Some("build") => return build(args[1..].to_vec()),
        Some("run") => return run_source(args[1..].to_vec()),
        _ => {}
    }
    let mut trace = false;
    let mut main = false;
    let mut rest = Vec::new();
    let mut opts_done = false;
    for a in args {
        if !opts_done && rest.is_empty() {
            match a.as_str() {
                "-t" | "--trace" => {
                    trace = true;
                    module::keep_scopes();
                    continue;
                }
                "-m" | "--main" => {
                    main = true;
                    continue;
                }
                "--" => {
                    opts_done = true;
                    continue;
                }
                _ => {}
            }
        }
        rest.push(a);
    }
    if rest.is_empty() {
        usage_error("the following arguments are required: module_file");
    }
    let module_file = rest.remove(0);
    let (func_name, program_args) = if main {
        ("__start__".to_string(), Some(rest))
    } else if rest.len() == 1 {
        (rest.remove(0), None)
    } else {
        usage_error("expected exactly one func_name (or use --main)");
    };

    let text = match std::fs::read_to_string(&module_file) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: [Errno {}] {}: '{}'", e.raw_os_error().unwrap_or(0), e, module_file);
            return 1;
        }
    };
    run_text(text, &module_file, func_name, program_args, trace)
}

/// Run the function func_name of the module text (module_file: its name for
/// the VProc), with program_args as a Seq of Strings if given.
fn run_text(text: String, module_file: &str, func_name: String, program_args: Option<Vec<String>>, trace: bool) -> i32 {
    let module_sexp = match sexp::parse(&text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let index = module::build_name_index(&module_sexp);
    let gid = match index.get(&func_name) {
        Some(g) => *g,
        None => {
            let mut names: Vec<&String> = index.keys().collect();
            names.sort();
            let avail: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
            eprintln!(
                "Error: function {} not found in module.\nAvailable: {}",
                value::py_repr(&func_name),
                avail.join(", ")
            );
            return 1;
        }
    };
    let prim_booleans = module::uses_boolean(&module_sexp);
    // a module of concise ConMa: __VProc_spawn__ makes modules of concise ConMa too
    prim::vprocs::set_concise(module::fast_features(&module_sexp).is_empty());
    let gvenv = match module::load_module_transpiled(&module_sexp, Some(transpiled::fnv1a(text.as_bytes()))) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error: {}", e);
            return 1;
        }
    };
    let closure = match gvenv.get(gid) {
        Some(c) => c,
        None => {
            eprintln!("Error: GID {} ({}) is undefined.", gid, value::py_repr(&func_name));
            return 1;
        }
    };
    let olist = match program_args {
        Some(a) => vec![Value::Seq(Rc::new(SeqClosure {
            olist: a.iter().map(|s| string_value(s)).collect(),
            sinfo: None,
            lvenv: None,
            scope: None,
        }))],
        None => Vec::new(),
    };
    transpiled::counts_from_env();
    let mut m = sched::Machine::new(trace);
    if prim_booleans {
        m.use_prim_booleans();
    }
    m.transpiled_on = !trace && m.profile.is_none() && std::env::var_os("CONMA_TRANSPILED_OFF").is_none();
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let vp = m.make_vproc(closure, olist, Some(module_file), gvenv, cwd);
    m.add(vp.clone());
    if trace {
        m.print_trace(&vp.vp.borrow(), 0);
    }
    let code = m.run();
    if let Some(p) = &m.profile {
        p.write();
    }
    transpiled::write_counts();
    if trace && code.is_none() {
        eprintln!("[done] normal termination after {} step(s).", m.steps);
    }
    // 1 if the first VProc ended with an error (the others do not count).
    code.unwrap_or(if m.main_failed { 1 } else { 0 })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // A large stack: modules are deeply nested, and values are dropped and
    // written recursively.
    let child = std::thread::Builder::new()
        .stack_size(1 << 30)
        .spawn(move || run(args))
        .expect("cannot start the VM thread");
    let code = child.join().unwrap_or(1);
    exit(code);
}
