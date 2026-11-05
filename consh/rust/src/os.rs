//! The OS functions that the standard library does not provide, and the
//! error texts of the ConMa VM (sys.rs), so that messages are the same.

use std::fs::File;
use std::io::{self, Write};
use std::os::fd::FromRawFd;
use std::os::raw::{c_char, c_int, c_long};
use std::sync::{Arc, Mutex};

extern "C" {
    fn sysconf(name: c_int) -> c_long;
    fn access(path: *const c_char, mode: c_int) -> c_int;
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
