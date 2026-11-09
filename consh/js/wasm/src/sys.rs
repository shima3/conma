//! The OS of the VM for wasm32: the same functions as vm/src/sys.rs, on a
//! virtual OS kept in memory (see 2.2 of consh/js/PLAN-ja.md).
//!
//! * fds 0, 1, 2: standard input is the bytes the host gives with
//!   conma_input (and conma_input_end); standard output and standard error go
//!   to the host's function host_write.
//! * Pipes are buffers in memory, without a limit of size, so that a write
//!   never waits.
//! * Files and directories are a table from normalized absolute paths to
//!   nodes; there are no symbolic links and no permissions.
//! * There are no processes and no signals; Ctrl-C of the host counts as a
//!   SIGINT (conma_interrupt).
//!
//! Nothing here blocks: a read with no data is EAGAIN, and the scheduler
//! returns to the host when every VProc waits (sched::Slice::Idle).

#![allow(dead_code)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::os::raw::{c_int, c_long};
use std::rc::Rc;

#[link(wasm_import_module = "env")]
extern "C" {
    /// Write len bytes at ptr to the host's standard output (fd 1) or
    /// standard error (fd 2).
    fn host_write(fd: i32, ptr: *const u8, len: usize);
    /// A read of standard input found it empty before its end: the host may
    /// give input now (conma_input, conma_input_end), if it can without
    /// waiting for events (the JavaScriptCore shell can; a browser cannot).
    fn host_input_needed();
    /// The file path (normalized, absolute) is not in the virtual file
    /// system: the host may put it there now (conma_put_file), as the
    /// JavaScriptCore shell does with the files of the real file system.
    fn host_file_needed(ptr: *const u8, len: usize);
    /// The directory path (normalized, absolute) is listed for the first
    /// time: the host may put its entries there now (conma_make_dirs,
    /// conma_put_unread), as Node.js does with the real file system.
    fn host_dir_needed(ptr: *const u8, len: usize);
    /// Start evaluating the JavaScript code with args (a JSON array of
    /// strings and arrays); the host writes the result to the pipe fd as lines
    /// of S-expressions (conma_write) and closes it (conma_close).
    fn host_js_call(fd: i32, job: i32, code_ptr: *const u8, code_len: usize, args_ptr: *const u8, args_len: usize);
    /// The VM wrote len bytes at ptr to the fd for the call job (the standard
    /// input of the code).
    fn host_js_input(job: i32, ptr: *const u8, len: usize);
    /// The VM closed the fd for the call job (the end of standard input).
    fn host_js_input_end(job: i32);
    /// The JS value numbered id (__JS_value__) as an S-expression of a value
    /// of ConSh, written to out (cap bytes); the length, or -errno.
    fn host_js_value(id_ptr: *const u8, id_len: usize, out: *mut u8, cap: usize) -> i32;

    // Files of the host (after conma_use_host_fs): paths are absolute and as
    // the VM has them (the host resolves . and .. and symbolic links), and a
    // negative result is minus an errno.
    /// Open the file path with mode b'r', b'w' (create, truncate) or b'a'
    /// (create, append); a directory is -EISDIR. Returns a file descriptor of
    /// the host.
    fn host_fs_open(path_ptr: *const u8, path_len: usize, mode: i32) -> i32;
    /// Read up to len bytes into ptr; 0 at the end.
    fn host_fs_read(fd: i32, ptr: *mut u8, len: usize) -> i32;
    /// Write len bytes from ptr; returns how many were written.
    fn host_fs_write(fd: i32, ptr: *const u8, len: usize) -> i32;
    fn host_fs_close(fd: i32) -> i32;
    /// 1 for a directory, 2 for another file (symbolic links followed).
    fn host_fs_stat(path_ptr: *const u8, path_len: usize) -> i32;
    /// Write the real path of path (UTF-8) to out, up to cap bytes; returns
    /// its length (which may be more than cap: then the call is made again).
    fn host_fs_realpath(path_ptr: *const u8, path_len: usize, out: *mut u8, cap: usize) -> i32;
    /// Write the names in the directory path, each followed by NUL, to out, as
    /// host_fs_realpath writes.
    fn host_fs_readdir(path_ptr: *const u8, path_len: usize, out: *mut u8, cap: usize) -> i32;

    // Processes of the host (__OS_spawn__ and the others).
    /// Start a process: argv and env (KEY=VALUE) are strings each followed by
    /// NUL; stdio points to 3 pairs (kind, number) for standard input, output
    /// and error: kind 0 the host's own fd number (0, 1, 2), 1 a file of the
    /// host (its fd), 2 the read end of a pipe of the VM (an fd of the VM that
    /// the host reads, conma_read, and closes), 3 the write end of a pipe of the
    /// VM (an fd of the VM that the host writes, conma_write, and closes), 4
    /// /dev/null; a file of the virtual file system is given as 2 or 3. detached: in a new session. Returns the number of the
    /// process, or minus an errno (-38, ENOSYS, if the host cannot start one).
    fn host_proc_spawn(
        argv_ptr: *const u8,
        argv_len: usize,
        env_ptr: *const u8,
        env_len: usize,
        cwd_ptr: *const u8,
        cwd_len: usize,
        stdio: *const i32,
        detached: i32,
    ) -> i32;
    /// The exit code of the process (minus the signal number if a signal
    /// killed it), or 999 while it runs.
    fn host_proc_poll(id: i32) -> i32;
    /// Send the signal; 0, or minus an errno.
    fn host_proc_kill(id: i32, sig: i32) -> i32;
    /// __OS_trap_interrupt__: from now on, a SIGINT of the host (or its
    /// Ctrl-C) is counted (conma_interrupt) instead of ending it.
    fn host_trap_sigint();
    /// __VProc_spawn__: make the module of the ConMa source file (relative to
    /// cwd) as conma build does. Writes to out, up to cap bytes, "0" and the
    /// module text, or "1" and the first line of the error; returns the
    /// length, which may be more than cap: then the call is made again with
    /// the same arguments, and the host gives the same text without making it
    /// again.
    fn host_build_module(src_ptr: *const u8, src_len: usize, cwd_ptr: *const u8, cwd_len: usize, out: *mut u8, cap: usize) -> i32;
}

/// A result of a host_fs_ function: the value, or the error of minus it.
fn host_result(r: i32) -> io::Result<i32> {
    if r < 0 {
        Err(err(-r))
    } else {
        Ok(r)
    }
}

/// The text that a host_fs_ function writes to a buffer (realpath, readdir).
fn host_text(f: impl Fn(*mut u8, usize) -> i32) -> io::Result<String> {
    let mut cap = 4096;
    loop {
        let mut buf = vec![0u8; cap];
        let n = host_result(f(buf.as_mut_ptr(), cap))? as usize;
        if n <= cap {
            buf.truncate(n);
            return Ok(String::from_utf8_lossy(&buf).into_owned());
        }
        cap = n;
    }
}

/// A file of the host, closed when its last fd in the VM is closed.
struct HostFile(i32);

impl Drop for HostFile {
    fn drop(&mut self) {
        unsafe { host_fs_close(self.0) };
    }
}

fn host_fs() -> bool {
    with(|os| os.host_fs)
}

/// Ask the host for the file path if it is missing or unread. An unread
/// file that the host does not give becomes empty.
fn fetch(path: &str) {
    let (missing, unread) = with(|os| match os.files.get(path) {
        None => (true, false),
        Some(Node::Unread) => (false, true),
        Some(_) => (false, false),
    });
    if missing || unread {
        unsafe { host_file_needed(path.as_ptr(), path.len()) };
    }
    if unread {
        with(|os| {
            if let Some(n @ Node::Unread) = os.files.get_mut(path) {
                *n = Node::File(Rc::new(RefCell::new(Vec::new())));
            }
        });
    }
}

pub const SIGINT: c_int = 2;

const EPERM: i32 = 1;
const ENOENT: i32 = 2;
const ESRCH: i32 = 3;
const EBADF: i32 = 9;
const EAGAIN: i32 = 35;
const ENOTDIR: i32 = 20;
const EISDIR: i32 = 21;
const EINVAL: i32 = 22;
const EPIPE: i32 = 32;
const EEXIST: i32 = 17;

fn err(errno: i32) -> io::Error {
    io::Error::from_raw_os_error(errno)
}

struct Pipe {
    buf: RefCell<VecDeque<u8>>,
    readers: Cell<usize>,
    writers: Cell<usize>,
}

#[derive(Clone)]
enum Fd {
    StdIn,
    StdOut,
    StdErr,
    PipeRead(Rc<Pipe>),
    PipeWrite(Rc<Pipe>),
    /// A file open for reading ('r'), writing ('w') or appending ('a').
    File { data: Rc<RefCell<Vec<u8>>>, pos: Rc<Cell<usize>>, mode: char },
    /// /dev/null
    Null,
    /// The write end of the standard input of a call of JavaScript, whose
    /// writes and close go to the host (host_js_input, host_js_input_end).
    ToHost(i32),
    /// A file of the host (conma_use_host_fs), and whether it was opened for
    /// reading.
    Host(Rc<HostFile>, bool),
}

enum Node {
    Dir,
    File(Rc<RefCell<Vec<u8>>>),
    /// A file whose contents the host gives when it is opened (see fetch).
    Unread,
    /// /dev/null: reads give nothing, writes are dropped.
    Null,
}

struct Os {
    fds: Vec<Option<Fd>>,
    stdin: VecDeque<u8>,
    stdin_end: bool,
    files: BTreeMap<String, Node>,
    env: Vec<(String, String)>,
    interrupts: u64,
    /// Standard input was found empty before its end (see wants_input).
    wants_input: bool,
    /// The directories that host_dir_needed was called for.
    listed: std::collections::HashSet<String>,
    /// A directory on the way of a path that the host does not give is
    /// taken to exist (for a host that cannot tell directories, see
    /// assume_dirs).
    assume_dirs: bool,
    /// The number of the last call of JavaScript.
    js_jobs: i32,
    /// Files are those of the host (conma_use_host_fs): open_file, is_dir,
    /// canonicalize and read_dir go to the host_fs_ functions.
    host_fs: bool,
    /// The number of processes that the host said have exited
    /// (conma_child_event): counted with the interrupts as signals.
    child_events: u64,
}

thread_local! {
    static OS: RefCell<Os> = RefCell::new(Os {
        fds: vec![Some(Fd::StdIn), Some(Fd::StdOut), Some(Fd::StdErr)],
        stdin: VecDeque::new(),
        stdin_end: false,
        files: [("/".to_string(), Node::Dir), ("/dev".to_string(), Node::Dir), ("/dev/null".to_string(), Node::Null)]
            .into_iter()
            .collect(),
        env: Vec::new(),
        interrupts: 0,
        wants_input: false,
        listed: std::collections::HashSet::new(),
        assume_dirs: false,
        js_jobs: 0,
        host_fs: false,
        child_events: 0,
    });
}

fn with<T>(f: impl FnOnce(&mut Os) -> T) -> T {
    OS.with(|os| f(&mut os.borrow_mut()))
}

impl Os {
    fn get(&self, fd: c_int) -> io::Result<Fd> {
        self.fds.get(fd as usize).and_then(|f| f.clone()).ok_or_else(|| err(EBADF))
    }

    /// Put f at the lowest free fd, as the OS does.
    fn add(&mut self, f: Fd) -> c_int {
        match f {
            Fd::PipeRead(ref p) => p.readers.set(p.readers.get() + 1),
            Fd::PipeWrite(ref p) => p.writers.set(p.writers.get() + 1),
            _ => {}
        }
        match self.fds.iter().position(|f| f.is_none()) {
            Some(i) => {
                self.fds[i] = Some(f);
                i as c_int
            }
            None => {
                self.fds.push(Some(f));
                (self.fds.len() - 1) as c_int
            }
        }
    }

    fn readable(&mut self, fd: c_int) -> io::Result<bool> {
        Ok(match self.get(fd)? {
            Fd::StdIn => {
                let ok = !self.stdin.is_empty() || self.stdin_end;
                self.wants_input |= !ok;
                ok
            }
            Fd::PipeRead(p) => !p.buf.borrow().is_empty() || p.writers.get() == 0,
            Fd::File { .. } | Fd::Null | Fd::Host(..) => true,
            _ => false,
        })
    }
}

// ── The functions of vm/src/sys.rs ──────────────────────────────────────────

pub fn make_pipe() -> io::Result<(c_int, c_int)> {
    let p = Rc::new(Pipe { buf: RefCell::new(VecDeque::new()), readers: Cell::new(0), writers: Cell::new(0) });
    Ok(with(|os| (os.add(Fd::PipeRead(p.clone())), os.add(Fd::PipeWrite(p)))))
}

pub fn dup(fd: c_int) -> io::Result<c_int> {
    with(|os| {
        let f = os.get(fd)?;
        Ok(os.add(f))
    })
}

pub fn close_fd(fd: c_int) -> io::Result<()> {
    let f = with(|os| {
        let f = os.get(fd)?;
        match &f {
            Fd::PipeRead(p) => p.readers.set(p.readers.get() - 1),
            Fd::PipeWrite(p) => p.writers.set(p.writers.get() - 1),
            _ => {}
        }
        os.fds[fd as usize] = None;
        Ok::<Fd, io::Error>(f)
    })?;
    if let Fd::ToHost(job) = f {
        unsafe { host_js_input_end(job) };
    }
    Ok(())
}

pub fn read_fd(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    // A read that the native VM makes without checking first (__OS_get_String__)
    // waits for input there: ask the host for it.
    if fd == 0 && with(|os| os.stdin.is_empty() && !os.stdin_end && matches!(os.get(0), Ok(Fd::StdIn))) {
        unsafe { host_input_needed() };
    }
    with(|os| match os.get(fd)? {
        Fd::StdIn => {
            if os.stdin.is_empty() {
                os.wants_input |= !os.stdin_end;
                return if os.stdin_end { Ok(0) } else { Err(err(EAGAIN)) };
            }
            let n = buf.len().min(os.stdin.len());
            for (i, b) in os.stdin.drain(..n).enumerate() {
                buf[i] = b;
            }
            Ok(n)
        }
        Fd::PipeRead(p) => {
            let mut q = p.buf.borrow_mut();
            if q.is_empty() {
                return if p.writers.get() == 0 { Ok(0) } else { Err(err(EAGAIN)) };
            }
            let n = buf.len().min(q.len());
            for (i, b) in q.drain(..n).enumerate() {
                buf[i] = b;
            }
            Ok(n)
        }
        Fd::Null => Ok(0),
        Fd::Host(f, true) => host_result(unsafe { host_fs_read(f.0, buf.as_mut_ptr(), buf.len()) }).map(|n| n as usize),
        Fd::File { data, pos, mode: 'r' } => {
            let d = data.borrow();
            let start = pos.get().min(d.len());
            let n = buf.len().min(d.len() - start);
            buf[..n].copy_from_slice(&d[start..start + n]);
            pos.set(start + n);
            Ok(n)
        }
        _ => Err(err(EBADF)),
    })
}

pub fn write_fd(fd: c_int, buf: &[u8]) -> io::Result<usize> {
    let f = with(|os| os.get(fd))?;
    match f {
        Fd::StdOut | Fd::StdErr => {
            unsafe { host_write(fd, buf.as_ptr(), buf.len()) };
            Ok(buf.len())
        }
        Fd::Null => Ok(buf.len()),
        Fd::Host(f, false) => host_result(unsafe { host_fs_write(f.0, buf.as_ptr(), buf.len()) }).map(|n| n as usize),
        Fd::ToHost(job) => {
            unsafe { host_js_input(job, buf.as_ptr(), buf.len()) };
            Ok(buf.len())
        }
        Fd::PipeWrite(p) => {
            if p.readers.get() == 0 {
                return Err(err(EPIPE));
            }
            p.buf.borrow_mut().extend(buf.iter().copied());
            Ok(buf.len())
        }
        Fd::File { data, pos, mode } if mode != 'r' => {
            let mut d = data.borrow_mut();
            if mode == 'a' {
                d.extend_from_slice(buf);
            } else {
                let start = pos.get();
                if d.len() < start {
                    d.resize(start, 0);
                }
                let end = start + buf.len();
                if d.len() < end {
                    d.resize(end, 0);
                }
                d[start..end].copy_from_slice(buf);
                pos.set(end);
            }
            Ok(buf.len())
        }
        _ => Err(err(EBADF)),
    }
}

/// The fds of reads that are readable and the fds of writes; never waits.
pub fn wait_fds(reads: &[c_int], writes: &[c_int], _timeout: Option<i32>) -> io::Result<Vec<c_int>> {
    with(|os| {
        let mut ready: Vec<c_int> = reads.iter().copied().filter(|&fd| os.readable(fd).unwrap_or(true)).collect();
        ready.extend_from_slice(writes);
        Ok(ready)
    })
}

pub fn readable(fd: c_int) -> io::Result<bool> {
    with(|os| os.readable(fd))
}

pub fn writable(fd: c_int) -> io::Result<bool> {
    with(|os| os.get(fd).map(|_| true))
}

pub fn send_signal(_pid: c_int, _sig: c_int) -> io::Result<()> {
    Err(err(ESRCH))
}

/// Start a process in the host (__OS_spawn__); its number in the host. fds
/// are the fds of the VM for its standard input, output and error (None: the
/// interpreter's own).
pub fn spawn(args: &[String], fds: &[Option<i32>], env: &[(String, String)], cwd: &str, detached: bool) -> io::Result<i32> {
    // The pipe ends that the host reads and writes are its own fds of the VM.
    let mut stdio: Vec<i32> = Vec::new();
    let mut taken: Vec<c_int> = Vec::new();
    let r = with(|os| {
        for (i, fd) in fds.iter().enumerate() {
            let (kind, n) = match fd {
                None => (0, i as i32),
                Some(fd) => match os.get(*fd)? {
                    Fd::StdIn => (0, 0),
                    Fd::StdOut => (0, 1),
                    Fd::StdErr => (0, 2),
                    Fd::Host(f, _) => (1, f.0),
                    f @ Fd::PipeRead(_) => {
                        let d = os.add(f);
                        taken.push(d);
                        (2, d)
                    }
                    f @ Fd::PipeWrite(_) => {
                        let d = os.add(f);
                        taken.push(d);
                        (3, d)
                    }
                    Fd::Null => (4, 0),
                    // A file of the virtual file system: as a pipe end, which the host reads or writes
                    f @ Fd::File { .. } => {
                        let reading = matches!(f, Fd::File { mode: 'r', .. });
                        let d = os.add(f);
                        taken.push(d);
                        (if reading { 2 } else { 3 }, d)
                    }
                    _ => return Err(err(EBADF)),
                },
            };
            stdio.push(kind);
            stdio.push(n);
        }
        Ok(())
    });
    let close_taken = |taken: &[c_int]| {
        for d in taken {
            let _ = close_fd(*d);
        }
    };
    if let Err(e) = r {
        close_taken(&taken);
        return Err(e);
    }
    let argv: String = args.iter().map(|a| format!("{}\0", a)).collect();
    let envs: String = env.iter().map(|(k, v)| format!("{}={}\0", k, v)).collect();
    let id = unsafe {
        host_proc_spawn(
            argv.as_ptr(),
            argv.len(),
            envs.as_ptr(),
            envs.len(),
            cwd.as_ptr(),
            cwd.len(),
            stdio.as_ptr(),
            detached as i32,
        )
    };
    if id < 0 {
        close_taken(&taken);
        return Err(err(-id));
    }
    Ok(id)
}

/// __VProc_spawn__: the module text of the source file, made by the host, or
/// the first line of the error.
pub fn build_module(source: &str, cwd: &str) -> Result<String, String> {
    let r = host_text(|out, cap| unsafe { host_build_module(source.as_ptr(), source.len(), cwd.as_ptr(), cwd.len(), out, cap) })
        .map_err(|e| strerror(&e))?;
    match r.split_at(r.len().min(1)) {
        ("0", text) => Ok(text.to_string()),
        (_, msg) => Err(msg.to_string()),
    }
}

/// The exit code of a process of the host, or None while it runs.
pub fn proc_poll(id: i32) -> io::Result<Option<i32>> {
    let r = unsafe { host_proc_poll(id) };
    Ok(if r == 999 { None } else { Some(r) })
}

pub fn proc_kill(id: i32, sig: c_int) -> io::Result<()> {
    host_result(unsafe { host_proc_kill(id, sig) }).map(|_| ())
}

pub fn js_call(code: &str, args_json: &str) -> io::Result<(c_int, c_int)> {
    let (r, w) = make_pipe()?;
    let (job, to_host) = with(|os| {
        os.js_jobs += 1;
        let job = os.js_jobs;
        (job, os.add(Fd::ToHost(job)))
    });
    unsafe { host_js_call(w, job, code.as_ptr(), code.len(), args_json.as_ptr(), args_json.len()) };
    Ok((r, to_host))
}

pub fn js_value(id: &str) -> io::Result<String> {
    host_text(|out, cap| unsafe { host_js_value(id.as_ptr(), id.len(), out, cap) })
}

pub fn setsid() -> c_int {
    -1
}

pub fn can_search(_path: &str) -> bool {
    true
}

pub fn sysconf_value(name: c_int) -> io::Result<c_long> {
    Ok(match name {
        1 => 1048576, // SC_ARG_MAX
        3 => 100,     // SC_CLK_TCK
        5 => 256,     // SC_OPEN_MAX
        29 => 65536,  // SC_PAGESIZE (of wasm)
        57 | 58 => 1, // SC_NPROCESSORS_CONF, SC_NPROCESSORS_ONLN
        _ => -1,
    })
}

/// The numbers of macOS, as the native VM on macOS has them.
pub fn sysconf_name(name: &str) -> Option<c_int> {
    let table: &[(&str, c_int)] = &[
        ("SC_ARG_MAX", 1),
        ("SC_CHILD_MAX", 2),
        ("SC_CLK_TCK", 3),
        ("SC_NGROUPS_MAX", 4),
        ("SC_OPEN_MAX", 5),
        ("SC_PAGESIZE", 29),
        ("SC_PAGE_SIZE", 29),
        ("SC_NPROCESSORS_CONF", 57),
        ("SC_NPROCESSORS_ONLN", 58),
    ];
    table.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

// ── Files ───────────────────────────────────────────────────────────────────

/// path made absolute (from /), without ., .. and repeated /.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for p in path.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(p),
        }
    }
    format!("/{}", parts.join("/"))
}

/// path as the OS resolves it (made absolute from /): every directory on the
/// way, also one that .. leaves, must exist (ENOENT) and be a directory
/// (ENOTDIR); "" is ENOENT. The last name need not exist.
fn resolve(path: &str) -> io::Result<String> {
    if path.is_empty() {
        return Err(err(ENOENT));
    }
    let mut parts: Vec<&str> = Vec::new();
    let mut first = true;
    for c in path.split('/').filter(|c| !c.is_empty() && *c != ".") {
        if !first {
            require_dir(&format!("/{}", parts.join("/")))?;
        }
        first = false;
        if c == ".." {
            parts.pop();
        } else {
            parts.push(c);
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

fn require_dir(path: &str) -> io::Result<()> {
    fetch(path);
    with(|os| match os.files.get(path) {
        Some(Node::Dir) => Ok(()),
        Some(_) => Err(err(ENOTDIR)),
        None if os.assume_dirs => Ok(()),
        None => Err(err(ENOENT)),
    })
}

fn parent(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => path[..i].to_string(),
    }
}

/// Check that the directory of path exists (ENOENT, ENOTDIR).
fn check_parent(os: &Os, path: &str) -> io::Result<()> {
    match os.files.get(&parent(path)) {
        Some(Node::Dir) => Ok(()),
        Some(_) => Err(err(ENOTDIR)),
        None => Err(err(ENOENT)),
    }
}

pub fn open_file(path: &str, mode: &str) -> io::Result<c_int> {
    if host_fs() {
        let m = mode.bytes().next().unwrap_or(b'r') as i32;
        let fd = host_result(unsafe { host_fs_open(path.as_ptr(), path.len(), m) })?;
        return Ok(with(|os| os.add(Fd::Host(Rc::new(HostFile(fd)), mode == "r"))));
    }
    let path = resolve(path)?;
    if mode != "w" {
        fetch(&path);
    }
    // A new file goes in a directory that the host may have.
    fetch(&parent(&path));
    with(|os| {
        let data = match os.files.get(&path) {
            Some(Node::Dir) => return Err(err(EISDIR)),
            Some(Node::Null) => return Ok(os.add(Fd::Null)),
            // Only "w" leaves a file unread (fetch reads it for "r" and "a"); it is emptied anyway.
            Some(Node::Unread) => {
                let d = Rc::new(RefCell::new(Vec::new()));
                os.files.insert(path.clone(), Node::File(d.clone()));
                d
            }
            Some(Node::File(d)) => {
                if mode == "w" {
                    d.borrow_mut().clear();
                }
                d.clone()
            }
            None if mode == "r" => return Err(err(ENOENT)),
            None => {
                check_parent(os, &path)?;
                let d = Rc::new(RefCell::new(Vec::new()));
                os.files.insert(path.clone(), Node::File(d.clone()));
                d
            }
        };
        let mode = mode.chars().next().unwrap_or('r');
        Ok(os.add(Fd::File { data, pos: Rc::new(Cell::new(0)), mode }))
    })
}

pub fn is_dir(path: &str) -> io::Result<bool> {
    if host_fs() {
        if path.is_empty() {
            return Err(err(ENOENT));
        }
        return host_result(unsafe { host_fs_stat(path.as_ptr(), path.len()) }).map(|k| k == 1);
    }
    let path = resolve(path)?;
    fetch(&path);
    with(|os| match os.files.get(&path) {
        Some(Node::Dir) => Ok(true),
        Some(_) => Ok(false),
        None => Err(err(ENOENT)),
    })
}

pub fn canonicalize(path: &str) -> io::Result<String> {
    if host_fs() {
        if path.is_empty() {
            return Err(err(ENOENT));
        }
        return host_text(|out, cap| unsafe { host_fs_realpath(path.as_ptr(), path.len(), out, cap) });
    }
    let p = resolve(path)?;
    is_dir(&p)?;
    Ok(p)
}

pub fn read_dir(path: &str) -> io::Result<Vec<String>> {
    if host_fs() {
        if path.is_empty() {
            return Err(err(ENOENT));
        }
        let names = host_text(|out, cap| unsafe { host_fs_readdir(path.as_ptr(), path.len(), out, cap) })?;
        return Ok(names.split('\0').filter(|n| !n.is_empty()).map(String::from).collect());
    }
    let path = resolve(path)?;
    fetch(&path);
    if with(|os| matches!(os.files.get(&path), Some(Node::Dir)) && os.listed.insert(path.clone())) {
        unsafe { host_dir_needed(path.as_ptr(), path.len()) };
    }
    with(|os| {
        match os.files.get(&path) {
            Some(Node::Dir) => {}
            Some(_) => return Err(err(ENOTDIR)),
            None => return Err(err(ENOENT)),
        }
        let prefix = if path == "/" { "/".to_string() } else { format!("{}/", path) };
        Ok(os
            .files
            .range(prefix.clone()..)
            .take_while(|(k, _)| k.starts_with(&prefix))
            .filter(|(k, _)| k.len() > prefix.len() && !k[prefix.len()..].contains('/'))
            .map(|(k, _)| k[prefix.len()..].to_string())
            .collect())
    })
}

pub fn env_vars() -> Vec<(String, String)> {
    with(|os| os.env.clone())
}

pub fn getpid() -> u32 {
    1
}

pub fn unsupported() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "not supported")
}

pub fn write_std(fd: c_int, text: &str) {
    let _ = write_fd(fd, text.as_bytes());
}

// ── Signals ─────────────────────────────────────────────────────────────────

/// No wakeup pipe: the scheduler does not block (Machine::blocking is false).
pub fn install_wakeup() -> io::Result<c_int> {
    Err(unsupported())
}

pub fn trap_sigint() {
    unsafe { host_trap_sigint() };
}

pub fn interrupts() -> u64 {
    with(|os| os.interrupts)
}

/// The interrupts and the exits of processes so far: the scheduler polls the
/// processes again only when this changes.
pub fn signals() -> u64 {
    with(|os| os.interrupts + os.child_events)
}

/// The host saw a process exit (or its output end): poll the processes again.
pub fn child_event() {
    with(|os| os.child_events += 1);
}

pub fn drain(_fd: c_int) {}

// ── Error messages as the native VM writes them (macOS) ─────────────────────

pub fn strerror(e: &io::Error) -> String {
    let text = match e.raw_os_error() {
        Some(EPERM) => "Operation not permitted",
        Some(ENOENT) => "No such file or directory",
        Some(7) => "Argument list too long",
        Some(38) => "Function not implemented",
        Some(ESRCH) => "No such process",
        Some(EBADF) => "Bad file descriptor",
        Some(13) => "Permission denied",
        Some(EEXIST) => "File exists",
        Some(ENOTDIR) => "Not a directory",
        Some(EISDIR) => "Is a directory",
        Some(EINVAL) => "Invalid argument",
        Some(EPIPE) => "Broken pipe",
        Some(EAGAIN) => "Resource temporarily unavailable",
        _ => return e.to_string(),
    };
    text.to_string()
}

pub fn errno_str(e: &io::Error) -> String {
    match e.raw_os_error() {
        Some(n) => format!("[Errno {}] {}", n, strerror(e)),
        None => e.to_string(),
    }
}

// ── For the host (src/lib.rs) ───────────────────────────────────────────────

pub fn push_input(bytes: &[u8]) {
    with(|os| os.stdin.extend(bytes.iter().copied()));
}

/// Whether a VProc found standard input empty since the last call: a
/// program that polls it (__OS_read__ gives "" when there is no data) keeps
/// running, so the host cannot wait for Slice::Idle to give more input.
pub fn wants_input() -> bool {
    with(|os| std::mem::replace(&mut os.wants_input, false))
}

pub fn end_input() {
    with(|os| os.stdin_end = true);
}

/// From now on, a directory on the way of a path that the host does not
/// give is taken to exist: for a host that can read files but cannot tell
/// directories (the JavaScriptCore shell).
pub fn assume_dirs() {
    with(|os| os.assume_dirs = true);
}

/// From now on, files are those of the host (the host_fs_ functions), not
/// the virtual file system: for a host that has a file system (Node.js).
pub fn use_host_fs() {
    with(|os| os.host_fs = true);
}

pub fn interrupt() {
    with(|os| os.interrupts += 1);
}

pub fn set_env(key: &str, value: &str) {
    with(|os| {
        os.env.retain(|(k, _)| k != key);
        os.env.push((key.to_string(), value.to_string()));
    });
}

/// Make the directory path and the directories above it.
pub fn make_dirs(path: &str) -> io::Result<()> {
    let path = normalize(path);
    with(|os| {
        let mut cur = String::new();
        for part in path.split('/').filter(|p| !p.is_empty()) {
            cur.push('/');
            cur.push_str(part);
            match os.files.get(&cur) {
                Some(Node::Dir) => {}
                Some(_) => return Err(err(ENOTDIR)),
                None => {
                    os.files.insert(cur.clone(), Node::Dir);
                }
            }
        }
        Ok(())
    })
}

/// Make the file path with data, and the directories above it.
pub fn put_file(path: &str, data: &[u8]) -> io::Result<()> {
    let path = normalize(path);
    make_dirs(&parent(&path))?;
    with(|os| match os.files.get(&path) {
        Some(Node::Dir) => Err(err(EISDIR)),
        _ => {
            os.files.insert(path, Node::File(Rc::new(RefCell::new(data.to_vec()))));
            Ok(())
        }
    })
}

/// Make the file path, whose contents the host gives when it is opened, and
/// the directories above it (a file that is already there stays).
pub fn put_unread(path: &str) -> io::Result<()> {
    let path = normalize(path);
    make_dirs(&parent(&path))?;
    with(|os| {
        os.files.entry(path).or_insert(Node::Unread);
    });
    Ok(())
}

/// The contents of the file path.
pub fn get_file(path: &str) -> Option<Vec<u8>> {
    with(|os| match os.files.get(&normalize(path)) {
        Some(Node::File(d)) => Some(d.borrow().clone()),
        _ => None,
    })
}
