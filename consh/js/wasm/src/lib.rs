//! The ConMa VM of vm/ for wasm32-unknown-unknown (see 2.2 of
//! consh/js/PLAN-ja.md). The modules of vm/src are used as they are, except
//! sys, which is the virtual OS of src/sys.rs.
//!
//! The host (consh/js/conma.js) gives strings as UTF-8 bytes in the memory
//! of the module (conma_alloc), and drives the VM:
//!
//! * conma_start(module, argv): load the module text and make the first
//!   VProc; argv is what conma (vm/src/main.rs) takes after the module text
//!   is read, NUL-separated: [-t] [-m] <module_file> <func_name | arg ...>.
//!   Returns 0, or the exit code of an error.
//! * conma_run(budget): run about budget steps. Returns -1 if some VProc can
//!   still run, -2 if every VProc waits for the outside (standard input, a
//!   Ctrl-C), or the exit code when the VM has ended.
//! * conma_input, conma_input_end, conma_interrupt: standard input and Ctrl-C;
//!   conma_wants_input says whether a VProc found standard input empty.
//! * conma_put_file, conma_put_unread, conma_make_dirs, conma_get_file, conma_setenv,
//!   conma_set_cwd: the virtual file system and the environment; the VM asks
//!   the host for a file it lacks (host_file_needed) and for the entries of
//!   a directory it lists for the first time (host_dir_needed).

// Parts of vm/src that only the native VM uses (processes, Machine::run).
#![allow(dead_code)]

#[path = "../../../../vm/src/alloc.rs"]
mod alloc;
#[path = "../../../../vm/src/gc.rs"]
mod gc;
#[path = "../../../../vm/src/module.rs"]
mod module;
#[path = "../../../../vm/src/prim/mod.rs"]
mod prim;
#[path = "../../../../vm/src/profile.rs"]
mod profile;
#[path = "../../../../vm/src/sched.rs"]
mod sched;
#[path = "../../../../vm/src/sexp.rs"]
mod sexp;
#[path = "../../../../vm/src/step.rs"]
mod step;
mod sys;
#[path = "../../../../vm/src/trace.rs"]
mod trace;
#[path = "../../../../vm/src/transpiled.rs"]
mod transpiled;
#[path = "../../../../vm/src/value.rs"]
mod value;
#[path = "../../../../vm/src/vproc.rs"]
mod vproc;

#[global_allocator]
static GLOBAL: alloc::Pool = alloc::Pool::new();

use std::cell::RefCell;
use std::rc::Rc;
use value::{string_value, SeqClosure, Value};

struct Vm {
    m: sched::Machine,
    trace: bool,
}

thread_local! {
    static VM: RefCell<Option<Vm>> = const { RefCell::new(None) };
    static CWD: RefCell<String> = RefCell::new("/".to_string());
}

fn eprint(text: &str) {
    sys::write_std(2, text);
}

/// The bytes the host put at ptr.
unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

unsafe fn text(ptr: *const u8, len: usize) -> String {
    String::from_utf8_lossy(bytes(ptr, len)).into_owned()
}

/// len bytes of memory for the host to write into.
#[no_mangle]
pub extern "C" fn conma_alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// ptr and len must be what conma_alloc returned and was given.
#[no_mangle]
pub unsafe extern "C" fn conma_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

fn start(module_text: String, args: Vec<String>) -> i32 {
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
        eprint("usage: conma [-t] [-m] module_file [func_name | arg ...]\nconma: error: the following arguments are required: module_file\n");
        return 2;
    }
    let module_file = rest.remove(0);
    let (func_name, program_args) = if main {
        ("__start__".to_string(), Some(rest))
    } else if rest.len() == 1 {
        (rest.remove(0), None)
    } else {
        eprint("usage: conma [-t] [-m] module_file [func_name | arg ...]\nconma: error: expected exactly one func_name (or use --main)\n");
        return 2;
    };
    let module_sexp = match sexp::parse(&module_text) {
        Ok(s) => s,
        Err(e) => {
            eprint(&format!("Error: {}\n", e));
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
            eprint(&format!(
                "Error: function {} not found in module.\nAvailable: {}\n",
                value::py_repr(&func_name),
                avail.join(", ")
            ));
            return 1;
        }
    };
    let gvenv = match module::load_module_transpiled(&module_sexp, Some(transpiled::fnv1a(module_text.as_bytes()))) {
        Ok(g) => g,
        Err(e) => {
            eprint(&format!("Error: {}\n", e));
            return 1;
        }
    };
    drop(module_sexp);
    let closure = match gvenv.get(gid) {
        Some(c) => c,
        None => {
            eprint(&format!("Error: GID {} ({}) is undefined.\n", gid, value::py_repr(&func_name)));
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
    m.blocking = false;
    // Run the transpiled code of the module, if the VM was built with some for it (as vm/src/main.rs
    // does, not with --trace)
    m.transpiled_on = !trace;
    m.seed();
    let cwd = CWD.with(|c| c.borrow().clone());
    let vp = m.make_vproc(closure, olist, Some(&module_file), gvenv, cwd);
    m.add(vp.clone());
    if trace {
        m.print_trace(&vp.vp.borrow(), 0);
    }
    VM.with(|v| *v.borrow_mut() = Some(Vm { m, trace }));
    0
}

/// # Safety
/// The pointers must point to len bytes each.
#[no_mangle]
pub unsafe extern "C" fn conma_start(module_ptr: *const u8, module_len: usize, argv_ptr: *const u8, argv_len: usize) -> i32 {
    let module_text = text(module_ptr, module_len);
    let argv = text(argv_ptr, argv_len);
    let args: Vec<String> = if argv.is_empty() { Vec::new() } else { argv.split('\0').map(String::from).collect() };
    start(module_text, args)
}

#[no_mangle]
pub extern "C" fn conma_run(budget: u32) -> i32 {
    VM.with(|v| {
        let mut slot = v.borrow_mut();
        let vm = match slot.as_mut() {
            Some(vm) => vm,
            None => return 1,
        };
        match vm.m.run_slice(budget as u64) {
            sched::Slice::Paused => -1,
            sched::Slice::Idle => -2,
            sched::Slice::Done(code) => {
                if let Some(p) = &vm.m.profile {
                    p.write();
                }
                if vm.trace && code.is_none() {
                    eprint(&format!("[done] normal termination after {} step(s).\n", vm.m.steps));
                }
                // 1 if the first VProc ended with an error (the others do not count).
                let code = code.unwrap_or(if vm.m.main_failed { 1 } else { 0 });
                *slot = None;
                code
            }
        }
    })
}

/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_input(ptr: *const u8, len: usize) {
    sys::push_input(bytes(ptr, len));
}

/// 1 if a VProc found standard input empty since the last call (a program
/// may poll it instead of waiting, so the host should give more input when
/// it has some, even while the VM is running).
#[no_mangle]
pub extern "C" fn conma_wants_input() -> i32 {
    sys::wants_input() as i32
}

#[no_mangle]
pub extern "C" fn conma_input_end() {
    sys::end_input();
}

/// Write len bytes at ptr to the fd (the pipe of __JS_call__). Returns 0, or
/// the errno of the error (EPIPE when the reader has closed it).
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_write(fd: i32, ptr: *const u8, len: usize) -> i32 {
    match sys::write_fd(fd, bytes(ptr, len)) {
        Ok(_) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

/// Read up to len bytes from the fd (a pipe end that the host was given by
/// host_proc_spawn) into ptr without waiting. Returns the number read (0 at
/// the end), or minus the errno (-35, EAGAIN, when there is nothing yet).
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_read(fd: i32, ptr: *mut u8, len: usize) -> i32 {
    let buf = if len == 0 { &mut [][..] } else { std::slice::from_raw_parts_mut(ptr, len) };
    match sys::read_fd(fd, buf) {
        Ok(n) => n as i32,
        Err(e) => -e.raw_os_error().unwrap_or(5),
    }
}

/// A process of the host has exited: the scheduler polls the processes again.
#[no_mangle]
pub extern "C" fn conma_child_event() {
    sys::child_event();
}

/// Close the fd (the pipe of __JS_call__). Returns 0, or the errno.
#[no_mangle]
pub extern "C" fn conma_close(fd: i32) -> i32 {
    match sys::close_fd(fd) {
        Ok(()) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

/// From now on, a directory on the way of a path that the host does not give
/// is taken to exist (for a host that cannot tell directories).
#[no_mangle]
pub extern "C" fn conma_assume_dirs() {
    sys::assume_dirs();
}

/// From now on, files are those of the host (the host_fs_ functions), not the
/// virtual file system (for a host that has a file system).
#[no_mangle]
pub extern "C" fn conma_use_host_fs() {
    sys::use_host_fs();
}

#[no_mangle]
pub extern "C" fn conma_interrupt() {
    sys::interrupt();
}

/// Returns 0, or the errno of the error.
///
/// # Safety
/// The pointers must point to len bytes each.
#[no_mangle]
pub unsafe extern "C" fn conma_put_file(path_ptr: *const u8, path_len: usize, data_ptr: *const u8, data_len: usize) -> i32 {
    match sys::put_file(&text(path_ptr, path_len), bytes(data_ptr, data_len)) {
        Ok(()) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

/// Returns 0, or the errno of the error.
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_make_dirs(ptr: *const u8, len: usize) -> i32 {
    match sys::make_dirs(&text(ptr, len)) {
        Ok(()) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

/// A file whose contents the host gives (conma_put_file) when the VM opens
/// it: the host lists files without reading them. Returns 0, or the errno.
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_put_unread(ptr: *const u8, len: usize) -> i32 {
    match sys::put_unread(&text(ptr, len)) {
        Ok(()) => 0,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

thread_local! {
    static RESULT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Put the contents of the file at conma_result_ptr; returns its length, or
/// -1 if there is no such file.
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_get_file(ptr: *const u8, len: usize) -> i32 {
    match sys::get_file(&text(ptr, len)) {
        Some(d) => {
            let n = d.len() as i32;
            RESULT.with(|r| *r.borrow_mut() = d);
            n
        }
        None => -1,
    }
}

#[no_mangle]
pub extern "C" fn conma_result_ptr() -> *const u8 {
    RESULT.with(|r| r.borrow().as_ptr())
}

/// # Safety
/// The pointers must point to len bytes each.
#[no_mangle]
pub unsafe extern "C" fn conma_setenv(key_ptr: *const u8, key_len: usize, value_ptr: *const u8, value_len: usize) {
    sys::set_env(&text(key_ptr, key_len), &text(value_ptr, value_len));
}

/// The current directory of the first VProc (before conma_start). Returns 0,
/// or the errno of the error.
///
/// # Safety
/// ptr must point to len bytes.
#[no_mangle]
pub unsafe extern "C" fn conma_set_cwd(ptr: *const u8, len: usize) -> i32 {
    match sys::canonicalize(&text(ptr, len)) {
        Ok(p) => {
            CWD.with(|c| *c.borrow_mut() = p);
            0
        }
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}
