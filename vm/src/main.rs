//! conma — run a ConMa VProc to completion, as test/vproc_run.py does.
//!
//! Usage:
//!     conma [-t|--trace] <module_file> <func_name>
//!     conma [-t|--trace] -m|--main <module_file> [<arg> ...]
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

fn run(args: Vec<String>) -> i32 {
    let mut trace = false;
    let mut main = false;
    let mut rest = Vec::new();
    let mut opts_done = false;
    for a in args {
        if !opts_done && rest.is_empty() {
            match a.as_str() {
                "-t" | "--trace" => {
                    trace = true;
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
    let gvenv = match module::load_module(&module_sexp) {
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
        }))],
        None => Vec::new(),
    };
    let mut m = sched::Machine::new(trace);
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let vp = m.make_vproc(closure, olist, Some(&module_file), gvenv, cwd);
    m.add(vp.clone());
    if trace {
        m.print_trace(&vp.vp.borrow(), 0);
    }
    let code = m.run();
    if let Some(p) = &m.profile {
        p.write();
    }
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
