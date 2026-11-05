//! The few C functions of the OS that the standard library does not provide.
//! Constants differ between macOS and Linux.

use std::io;
use std::os::raw::{c_int, c_long, c_short};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

#[cfg(target_os = "linux")]
type NfdsT = std::os::raw::c_ulong;
#[cfg(not(target_os = "linux"))]
type NfdsT = std::os::raw::c_uint;

extern "C" {
    fn pipe(fds: *mut c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut std::ffi::c_void, n: usize) -> isize;
    #[link_name = "write"]
    fn c_write(fd: c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
    fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
    fn poll(fds: *mut PollFd, n: NfdsT, timeout: c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    pub fn setsid() -> c_int;
    fn access(path: *const std::os::raw::c_char, mode: c_int) -> c_int;
    fn sysconf(name: c_int) -> c_long;
}

const F_SETFD: c_int = 2;
const FD_CLOEXEC: c_int = 1;
#[cfg(target_os = "linux")]
const F_DUPFD_CLOEXEC: c_int = 1030;
#[cfg(not(target_os = "linux"))]
const F_DUPFD_CLOEXEC: c_int = 67;
const POLLIN: c_short = 1;
const POLLOUT: c_short = 4;
pub const SIGINT: c_int = 2;
#[cfg(target_os = "linux")]
const SIGCHLD: c_int = 17;
#[cfg(not(target_os = "linux"))]
const SIGCHLD: c_int = 20;

fn check(r: c_int) -> io::Result<c_int> {
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(r)
    }
}

/// A pipe whose fds are not inherited by child processes, like os.pipe().
pub fn make_pipe() -> io::Result<(c_int, c_int)> {
    let mut fds = [0 as c_int; 2];
    check(unsafe { pipe(fds.as_mut_ptr()) })?;
    for fd in fds {
        unsafe { fcntl(fd, F_SETFD, FD_CLOEXEC) };
    }
    Ok((fds[0], fds[1]))
}

/// dup, not inherited by child processes, like os.dup().
pub fn dup(fd: c_int) -> io::Result<c_int> {
    check(unsafe { fcntl(fd, F_DUPFD_CLOEXEC, 0 as c_int) })
}

pub fn close_fd(fd: c_int) -> io::Result<()> {
    check(unsafe { close(fd) }).map(|_| ())
}

pub fn read_fd(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n >= 0 {
            return Ok(n as usize);
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

pub fn write_fd(fd: c_int, buf: &[u8]) -> io::Result<usize> {
    loop {
        let n = unsafe { c_write(fd, buf.as_ptr().cast(), buf.len()) };
        if n >= 0 {
            return Ok(n as usize);
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}

/// Wait until some of the fds are ready, as select(): reads are the fds to
/// read, writes the fds to write; timeout None blocks. Returns the ready fds.
pub fn wait_fds(reads: &[c_int], writes: &[c_int], timeout: Option<i32>) -> io::Result<Vec<c_int>> {
    let mut fds: Vec<PollFd> = reads
        .iter()
        .map(|&fd| PollFd { fd, events: POLLIN, revents: 0 })
        .chain(writes.iter().map(|&fd| PollFd { fd, events: POLLOUT, revents: 0 }))
        .collect();
    loop {
        let r = unsafe { poll(fds.as_mut_ptr(), fds.len() as NfdsT, timeout.unwrap_or(-1)) };
        if r >= 0 {
            break;
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
    Ok(fds.iter().filter(|p| p.revents != 0).map(|p| p.fd).collect())
}

/// True if fd is ready to read (or at EOF) without blocking.
pub fn readable(fd: c_int) -> io::Result<bool> {
    Ok(!wait_fds(&[fd], &[], Some(0))?.is_empty())
}

/// True if fd can be written without blocking.
pub fn writable(fd: c_int) -> io::Result<bool> {
    Ok(!wait_fds(&[], &[fd], Some(0))?.is_empty())
}

pub fn send_signal(pid: c_int, sig: c_int) -> io::Result<()> {
    check(unsafe { kill(pid, sig) }).map(|_| ())
}

pub fn can_search(path: &str) -> bool {
    let c = match std::ffi::CString::new(path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    unsafe { access(c.as_ptr(), 1) == 0 }
}

pub fn sysconf_value(name: c_int) -> io::Result<c_long> {
    Ok(unsafe { sysconf(name) })
}

/// The numbers of the sysconf names that Python's os.sysconf_names knows.
pub fn sysconf_name(name: &str) -> Option<c_int> {
    #[cfg(not(target_os = "linux"))]
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
    #[cfg(target_os = "linux")]
    let table: &[(&str, c_int)] = &[
        ("SC_ARG_MAX", 0),
        ("SC_CHILD_MAX", 1),
        ("SC_CLK_TCK", 2),
        ("SC_NGROUPS_MAX", 3),
        ("SC_OPEN_MAX", 4),
        ("SC_PAGESIZE", 30),
        ("SC_PAGE_SIZE", 30),
        ("SC_NPROCESSORS_CONF", 83),
        ("SC_NPROCESSORS_ONLN", 84),
    ];
    table.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

// ── Signals ─────────────────────────────────────────────────────────────────

/// Write end of the pipe that signals write to, so that a wait for fds wakes up.
static WAKE_FD: AtomicI32 = AtomicI32::new(-1);
/// The number of SIGINTs since __OS_trap_interrupt__.
static INTERRUPTS: AtomicU64 = AtomicU64::new(0);

extern "C" fn on_sigchld(_sig: c_int) {
    let fd = WAKE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        unsafe { c_write(fd, b"x".as_ptr().cast(), 1) };
    }
}

extern "C" fn on_sigint(sig: c_int) {
    INTERRUPTS.fetch_add(1, Ordering::SeqCst);
    on_sigchld(sig);
}

/// Make SIGCHLD write a byte to a pipe; returns the read end.
pub fn install_wakeup() -> io::Result<c_int> {
    let (r, w) = make_pipe()?;
    for fd in [r, w] {
        // O_NONBLOCK: F_GETFL 3, F_SETFL 4
        let fl = unsafe { fcntl(fd, 3) };
        #[cfg(target_os = "linux")]
        let nb = 0o4000;
        #[cfg(not(target_os = "linux"))]
        let nb = 4;
        unsafe { fcntl(fd, 4, fl | nb) };
    }
    WAKE_FD.store(w, Ordering::SeqCst);
    unsafe { signal(SIGCHLD, on_sigchld as *const () as usize) };
    Ok(r)
}

/// Count SIGINTs instead of letting them stop the VM.
pub fn trap_sigint() {
    unsafe { signal(SIGINT, on_sigint as *const () as usize) };
}

pub fn interrupts() -> u64 {
    INTERRUPTS.load(Ordering::SeqCst)
}

/// Read and drop what the signals wrote to the wakeup pipe.
pub fn drain(fd: c_int) {
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            break;
        }
    }
}

// ── Error messages as Python writes them ────────────────────────────────────

/// e.strerror: the text of the C strerror.
pub fn strerror(e: &io::Error) -> String {
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

/// str(e) of an OSError without a file name: "[Errno 9] Bad file descriptor".
pub fn errno_str(e: &io::Error) -> String {
    match e.raw_os_error() {
        Some(n) => format!("[Errno {}] {}", n, strerror(e)),
        None => e.to_string(),
    }
}
