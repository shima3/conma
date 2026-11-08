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
    fn host_js_call(fd: i32, code_ptr: *const u8, code_len: usize, args_ptr: *const u8, args_len: usize);
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
            Fd::File { .. } | Fd::Null => true,
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
    with(|os| {
        let f = os.get(fd)?;
        match f {
            Fd::PipeRead(p) => p.readers.set(p.readers.get() - 1),
            Fd::PipeWrite(p) => p.writers.set(p.writers.get() - 1),
            _ => {}
        }
        os.fds[fd as usize] = None;
        Ok(())
    })
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

pub fn js_call(code: &str, args_json: &str) -> io::Result<c_int> {
    let (r, w) = make_pipe()?;
    unsafe { host_js_call(w, code.as_ptr(), code.len(), args_json.as_ptr(), args_json.len()) };
    Ok(r)
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
    let path = resolve(path)?;
    fetch(&path);
    with(|os| match os.files.get(&path) {
        Some(Node::Dir) => Ok(true),
        Some(_) => Ok(false),
        None => Err(err(ENOENT)),
    })
}

pub fn canonicalize(path: &str) -> io::Result<String> {
    let p = resolve(path)?;
    is_dir(&p)?;
    Ok(p)
}

pub fn read_dir(path: &str) -> io::Result<Vec<String>> {
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

pub fn trap_sigint() {}

pub fn interrupts() -> u64 {
    with(|os| os.interrupts)
}

pub fn signals() -> u64 {
    interrupts()
}

pub fn drain(_fd: c_int) {}

// ── Error messages as the native VM writes them (macOS) ─────────────────────

pub fn strerror(e: &io::Error) -> String {
    let text = match e.raw_os_error() {
        Some(EPERM) => "Operation not permitted",
        Some(ENOENT) => "No such file or directory",
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
