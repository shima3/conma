//! The OS functions that the standard library does not provide, and the
//! error texts of the ConMa VM (sys.rs), so that messages are the same.

use std::fs::File;
use std::io::{self, Write};
use std::os::fd::FromRawFd;
use std::os::raw::{c_char, c_int, c_long};
use std::sync::{Arc, Mutex};

extern "C" {
    #[link_name = "setsid"]
    fn c_setsid() -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn sysconf(name: c_int) -> c_long;
    fn access(path: *const c_char, mode: c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
    fn write(fd: c_int, buf: *const std::ffi::c_void, n: usize) -> isize;
}

static INTERRUPT_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

extern "C" fn on_interrupt(_sig: c_int) {
    let fd = INTERRUPT_FD.load(std::sync::atomic::Ordering::Relaxed);
    if fd >= 0 {
        unsafe {
            write(fd, b"i".as_ptr() as *const std::ffi::c_void, 1);
        }
    }
}

/// SIGINT does not end the process: each one writes a byte to a pipe, whose
/// reading end this returns (interactive mode, __OS_trap_interrupt__).
pub fn trap_interrupt() -> File {
    use std::os::fd::IntoRawFd;
    let (r, w) = io::pipe().expect("cannot make a pipe");
    INTERRUPT_FD.store(w.into_raw_fd(), std::sync::atomic::Ordering::Relaxed);
    unsafe {
        signal(2, on_interrupt as *const () as usize);
    }
    File::from(std::os::fd::OwnedFd::from(r))
}

#[cfg(target_os = "linux")]
const SC_ARG_MAX: c_int = 0;
#[cfg(not(target_os = "linux"))]
const SC_ARG_MAX: c_int = 1;

pub fn arg_max() -> i64 {
    unsafe { sysconf(SC_ARG_MAX) as i64 }
}

/// The message of an OS error without " (os error N)".
pub fn strerror(e: &io::Error) -> String {
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

pub fn io_error(errno: i32) -> io::Error {
    io::Error::from_raw_os_error(errno)
}

/// Whether the directory can be searched (X_OK).
pub fn can_search(path: &std::path::Path) -> bool {
    let c = match std::ffi::CString::new(path.as_os_str().as_encoded_bytes()) {
        Ok(c) => c,
        Err(_) => return false,
    };
    unsafe { access(c.as_ptr(), 1) == 0 }
}

/// An open file, shared by the targets and inputs that use it. It is closed
/// when the last of them is dropped, except the standard streams.
pub struct Fd {
    pub file: Mutex<Option<File>>,
    pub raw: c_int,
}

impl Fd {
    pub fn new(file: File) -> Arc<Fd> {
        use std::os::fd::AsRawFd;
        let raw = file.as_raw_fd();
        Arc::new(Fd { file: Mutex::new(Some(file)), raw })
    }
    /// fd 0, 1 or 2 of the shell, never closed.
    pub fn standard(raw: c_int) -> Arc<Fd> {
        let file = unsafe { File::from_raw_fd(raw) };
        let fd = Arc::new(Fd { file: Mutex::new(Some(file)), raw });
        // Keep the File from closing the fd.
        std::mem::forget(fd.clone());
        fd
    }
    pub fn write_all(&self, data: &[u8]) -> io::Result<()> {
        let mut g = self.file.lock().unwrap();
        match g.as_mut() {
            Some(f) => f.write_all(data),
            None => Err(io_error(9)),
        }
    }
    /// Close the file now (a second time does nothing).
    pub fn close(&self) {
        if self.raw > 2 {
            self.file.lock().unwrap().take();
        }
    }
    /// A new fd for the same file (for a child process).
    pub fn dup(&self) -> io::Result<std::os::fd::OwnedFd> {
        let g = self.file.lock().unwrap();
        match g.as_ref() {
            Some(f) => Ok(f.try_clone()?.into()),
            None => Err(io_error(9)),
        }
    }
}

impl Drop for Fd {
    fn drop(&mut self) {
        if self.raw <= 2 {
            if let Some(f) = self.file.get_mut().unwrap().take() {
                std::mem::forget(f);
            }
        }
    }
}

/// Write to fd 2 at once (as __OS_print_error__).
pub fn print_error(s: &str) {
    let _ = io::stderr().write_all(s.as_bytes());
}

/// Start a new session (in a child before exec).
pub fn setsid() {
    unsafe {
        c_setsid();
    }
}

/// Send SIGTERM to a process.
pub fn terminate(pid: u32) {
    unsafe {
        kill(pid as c_int, 15);
    }
}

/// With CONSH_JITTER=<seed>, a short random pause or a yield at the points
/// where threads meet (channels, writes, the start of threads, statements),
/// so that tests run with many orders of the threads and show races.
pub fn jitter() {
    use std::cell::Cell;
    use std::sync::OnceLock;
    static SEED: OnceLock<Option<u64>> = OnceLock::new();
    let seed = match SEED.get_or_init(|| std::env::var("CONSH_JITTER").ok().and_then(|s| s.parse().ok())) {
        None => return,
        Some(s) => *s,
    };
    thread_local! { static STATE: Cell<u64> = const { Cell::new(0) }; }
    let r = STATE.with(|st| {
        let mut x = st.get();
        if x == 0 {
            let t = std::thread::current().id();
            let h = format!("{:?}", t).bytes().fold(seed ^ 0x9e37_79b9_7f4a_7c15, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
            x = h | 1;
        }
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        st.set(x);
        x
    });
    match r % 8 {
        0 => std::thread::sleep(std::time::Duration::from_micros(r % 2000)),
        1 | 2 => std::thread::yield_now(),
        _ => {}
    }
}
