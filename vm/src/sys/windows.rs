//! The OS functions of the native VM on Windows (docs/dev/PLATFORM-PLAN-ja.md 4.11), with the
//! same interface as unix.rs. Windows has no integer fds, no poll() for anonymous pipes and no
//! signals, so this file keeps:
//!
//! * a table from fds to what they are: 0, 1 and 2 are the standard handles, and a new fd is the
//!   lowest free number, as on Unix;
//! * for a pipe that the VM reads, PeekNamedPipe in place of poll(): it reads nothing ahead, so
//!   that what the VM does not read is left for a child process that gets the same handle (the
//!   standard input of ConSh); wait_fds looks at such pipes again every few milliseconds;
//! * a reader thread for a console that the VM reads (it cannot be peeked), which reads only when
//!   the VM asks (one ReadFile of up to 4,096 bytes), and is stopped before a child process gets
//!   the console;
//! * a writer thread for each pipe that the VM writes, with a buffer of 64 KiB, so that a write
//!   never blocks the VM while its reader is a VProc of the same VM; a write waits a little for
//!   its bytes to reach the pipe, so that they can be read at once, and a broken pipe (EPIPE) is
//!   mostly found by the write that meets it, as on Unix;
//! * a pipe in memory for the wakeup pipe, and one Condvar that wakes wait_fds;
//! * a thread for each child process that waits for it to exit, in place of SIGCHLD;
//! * Ctrl-C by SetConsoleCtrlHandler;
//! * error texts as Python writes them on macOS and Linux ([Errno 2] No such file or directory).

use std::collections::VecDeque;
use std::ffi::c_void;
use std::io;
use std::os::raw::{c_int, c_long};
use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicI32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

type Handle = isize;
type Bool = i32;

#[repr(C)]
struct SystemInfo {
    oem_id: u32,
    page_size: u32,
    min_address: *mut c_void,
    max_address: *mut c_void,
    active_processor_mask: usize,
    number_of_processors: u32,
    processor_type: u32,
    allocation_granularity: u32,
    processor_level: u16,
    processor_revision: u16,
}

#[link(name = "kernel32")]
extern "system" {
    fn CreatePipe(r: *mut Handle, w: *mut Handle, attrs: *const c_void, size: u32) -> Bool;
    fn CloseHandle(h: Handle) -> Bool;
    fn ReadFile(h: Handle, buf: *mut u8, n: u32, done: *mut u32, overlapped: *mut c_void) -> Bool;
    fn WriteFile(h: Handle, buf: *const u8, n: u32, done: *mut u32, overlapped: *mut c_void) -> Bool;
    fn GetStdHandle(n: u32) -> Handle;
    fn GetFileType(h: Handle) -> u32;
    fn GetCurrentProcess() -> Handle;
    fn DuplicateHandle(sp: Handle, s: Handle, tp: Handle, t: *mut Handle, access: u32, inherit: Bool, options: u32) -> Bool;
    fn CancelSynchronousIo(thread: Handle) -> Bool;
    fn WaitForSingleObject(h: Handle, ms: u32) -> u32;
    fn TerminateProcess(h: Handle, code: u32) -> Bool;
    fn SetConsoleCtrlHandler(f: Option<unsafe extern "system" fn(u32) -> Bool>, add: Bool) -> Bool;
    fn GetSystemInfo(info: *mut SystemInfo);
    fn PeekNamedPipe(h: Handle, buf: *mut u8, n: u32, read: *mut u32, avail: *mut u32, left: *mut u32) -> Bool;
    fn SetLastError(code: u32);
    fn GetLastError() -> u32;
    fn SetHandleInformation(h: Handle, mask: u32, flags: u32) -> Bool;
    fn GetConsoleMode(h: Handle, mode: *mut u32) -> Bool;
    fn SetConsoleMode(h: Handle, mode: u32) -> Bool;
}

const FILE_TYPE_DISK: u32 = 1;
const FILE_TYPE_CHAR: u32 = 2;
const FILE_TYPE_PIPE: u32 = 3;
const DUPLICATE_SAME_ACCESS: u32 = 2;
const INFINITE: u32 = 0xFFFF_FFFF;
const ERROR_INVALID_HANDLE: i32 = 6;
const ERROR_HANDLE_EOF: i32 = 38;
const ERROR_BROKEN_PIPE: i32 = 109;
const ERROR_OPERATION_ABORTED: i32 = 995;
/// CTRL_C_EVENT and CTRL_BREAK_EVENT.
const CTRL_BREAK_EVENT: u32 = 1;
/// The exit code of a process that Ctrl-C ended (STATUS_CONTROL_C_EXIT).
const CONTROL_C_EXIT: u32 = 0xC000_013A;
/// TerminateProcess gives the exit code KILLED + the signal number (__OS_kill__).
const KILLED: u32 = 0x4B49_4C00;
pub const SIGINT: c_int = 2;
/// How much a writer thread keeps (the size of a pipe on Unix), and the size of a pipe.
const WRITE_BUFFER: usize = 65536;
/// How long a write to a pipe whose writer thread is idle waits for its bytes to reach the pipe
/// (they do at once unless the pipe is full; long enough for a loaded machine).
const WRITE_THROUGH: Duration = Duration::from_millis(200);
/// When the VM ends, how long a writer thread may write nothing before the VM stops waiting for
/// it (the reader has stopped reading).
const FLUSH_STALL: Duration = Duration::from_secs(1);
/// How often wait_fds looks at a pipe that it waits to read.
const PEEK_EVERY: Duration = Duration::from_millis(2);
/// After a read of a console gives 0 bytes, how long the reader waits for a Ctrl-C to be counted
/// before it takes the 0 bytes for the end of the input. A Ctrl-C typed in a pseudo console (as in
/// a terminal of Windows Terminal or of an editor) stops the read with 0 bytes and no error, and
/// the system calls the handler on a thread of its own, which can count it a little later.
const CTRL_C_WAIT: Duration = Duration::from_millis(100);
/// How much a reader thread reads at once.
const READ_CHUNK: usize = 4096;

fn last_error() -> io::Error {
    io::Error::last_os_error()
}

fn bad_fd() -> io::Error {
    io::Error::from_raw_os_error(ERROR_INVALID_HANDLE)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// ── Waking wait_fds ─────────────────────────────────────────────────────────

/// Counts the changes that may make an fd ready; wait_fds waits on COND for it to change.
static CHANGES: Mutex<u64> = Mutex::new(0);
static COND: Condvar = Condvar::new();

fn notify() {
    *lock(&CHANGES) += 1;
    COND.notify_all();
}

// ── Reader threads ──────────────────────────────────────────────────────────

struct Reader {
    handle: Handle,
    owned: bool,
    /// a console or a character device: 0 bytes is the end, and "\r\n" is "\n"
    chars: bool,
    /// of a console, its input mode when the reader was made. A program that the VM runs may
    /// change it (a program of MSYS, as those of Git Bash, turns off line input, echo and Ctrl-C
    /// while it runs), and one that is stopped (Ctrl-C, __OS_kill__) cannot set it back; then
    /// Enter would give "\r" alone and no line would end. Each read of the console sets it back.
    console_mode: Option<u32>,
    st: Mutex<ReadState>,
    cv: Condvar,
    thread: Mutex<Option<JoinHandle<()>>>,
}

#[derive(Default)]
struct ReadState {
    buf: VecDeque<u8>,
    eof: bool,
    err: Option<i32>,
    /// the VM asked for a read that has not been started
    want: bool,
    /// a ReadFile is in progress
    pending: bool,
    /// the fd has been closed: the thread ends
    quit: bool,
    /// a console read ended with "\r": dropped if the next one begins with "\n"
    cr: bool,
}

impl Reader {
    fn new(handle: Handle, owned: bool, chars: bool) -> Arc<Reader> {
        let mut mode = 0u32;
        let console_mode = if chars && unsafe { GetConsoleMode(handle, &mut mode) } != 0 { Some(mode) } else { None };
        Arc::new(Reader { handle, owned, chars, console_mode, st: Mutex::new(ReadState::default()), cv: Condvar::new(), thread: Mutex::new(None) })
    }

    /// Set the input mode of the console back to the one it had when the reader was made.
    fn restore_console_mode(&self) {
        if let Some(saved) = self.console_mode {
            let mut now = 0u32;
            if unsafe { GetConsoleMode(self.handle, &mut now) } != 0 && now != saved {
                unsafe { SetConsoleMode(self.handle, saved) };
            }
        }
    }

    /// Ask for one ReadFile if nothing is there and none is in progress.
    fn request(self: &Arc<Self>, st: &mut ReadState) {
        if !st.buf.is_empty() || st.eof || st.err.is_some() || st.pending || st.want || st.quit {
            return;
        }
        st.want = true;
        let mut t = lock(&self.thread);
        if t.is_none() {
            let r = self.clone();
            *t = Some(std::thread::spawn(move || r.run()));
        }
        self.cv.notify_all();
    }

    fn run(self: Arc<Self>) {
        let mut chunk = vec![0u8; READ_CHUNK];
        loop {
            let mut st = lock(&self.st);
            while !st.want && !st.quit {
                st = self.cv.wait(st).unwrap_or_else(|e| e.into_inner());
            }
            if st.quit {
                return;
            }
            st.want = false;
            st.pending = true;
            drop(st);
            self.restore_console_mode();
            let mut n = 0u32;
            let interrupts_before = INTERRUPTS.load(Ordering::SeqCst);
            unsafe { SetLastError(0) };
            let ok = unsafe { ReadFile(self.handle, chunk.as_mut_ptr(), READ_CHUNK as u32, &mut n, std::ptr::null_mut()) } != 0;
            let code = if ok { unsafe { GetLastError() as i32 } } else { last_error().raw_os_error().unwrap_or(0) };
            // A console gives 0 bytes at the end, and also when Ctrl-C stops the read: with
            // ERROR_OPERATION_ABORTED while the handler is in, or with no error from a pseudo
            // console, where only the count of Ctrl-Cs tells them apart
            let ended = ok && n == 0 && self.chars && code != ERROR_OPERATION_ABORTED && !ctrl_c_since(interrupts_before);
            let mut st = lock(&self.st);
            st.pending = false;
            if ok {
                if n == 0 {
                    // a pipe gives 0 bytes for a write of 0 bytes, and ERROR_BROKEN_PIPE at the end.
                    // A console: nothing was read, and the input has ended unless a Ctrl-C stopped the read
                    if ended {
                        if std::mem::take(&mut st.cr) {
                            st.buf.push_back(b'\r');
                        }
                        st.eof = true;
                    }
                } else if self.chars {
                    // "\r\n" is "\n", also when the two come in two reads
                    for &b in &chunk[..n as usize] {
                        if std::mem::take(&mut st.cr) && b != b'\n' {
                            st.buf.push_back(b'\r');
                        }
                        if b == b'\r' {
                            st.cr = true;
                        } else {
                            st.buf.push_back(b);
                        }
                    }
                } else {
                    st.buf.extend(&chunk[..n as usize]);
                }
            } else if code == ERROR_OPERATION_ABORTED {
                // stopped by cancel(): nothing was read
            } else if code == ERROR_BROKEN_PIPE || code == ERROR_HANDLE_EOF {
                st.eof = true;
            } else {
                st.err = Some(code);
            }
            drop(st);
            self.cv.notify_all();
            notify();
        }
    }

    /// Ready to read without blocking; if not, start a read.
    fn ready(self: &Arc<Self>) -> bool {
        let mut st = lock(&self.st);
        if !st.buf.is_empty() || st.eof || st.err.is_some() {
            return true;
        }
        self.request(&mut st);
        false
    }

    fn read(self: &Arc<Self>, out: &mut [u8]) -> io::Result<usize> {
        let mut st = lock(&self.st);
        loop {
            if !st.buf.is_empty() {
                let n = out.len().min(st.buf.len());
                for (o, b) in out.iter_mut().zip(st.buf.drain(..n)) {
                    *o = b;
                }
                return Ok(n);
            }
            if let Some(e) = st.err {
                return Err(io::Error::from_raw_os_error(e));
            }
            if st.eof || out.is_empty() {
                return Ok(0);
            }
            self.request(&mut st);
            st = self.cv.wait(st).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Stop a ReadFile in progress (before a child process gets the handle, or it is closed);
    /// with quit, end the thread too.
    fn cancel(&self, quit: bool) {
        let mut st = lock(&self.st);
        st.want = false;
        st.quit |= quit;
        self.cv.notify_all();
        while st.pending {
            drop(st);
            if let Some(t) = lock(&self.thread).as_ref() {
                // fails if the thread has not entered ReadFile yet: try again
                unsafe { CancelSynchronousIo(t.as_raw_handle() as Handle) };
            }
            st = lock(&self.st);
            if st.pending {
                st = self.cv.wait_timeout(st, Duration::from_millis(1)).unwrap_or_else(|e| e.into_inner()).0;
            }
        }
    }

    fn close(&self) {
        self.cancel(true);
        if self.owned {
            unsafe { CloseHandle(self.handle) };
        }
    }
}

// ── Writer threads ──────────────────────────────────────────────────────────

struct Writer {
    handle: Handle,
    st: Mutex<WriteState>,
    cv: Condvar,
}

#[derive(Default)]
struct WriteState {
    buf: VecDeque<u8>,
    err: Option<i32>,
    /// the thread is writing what it took from buf
    busy: bool,
    started: bool,
    /// the fd has been closed: write out buf, close the handle and end
    closing: bool,
    /// the handle has been closed
    done: bool,
    /// the number of pieces written: flush sees whether the thread still gets on
    written: u64,
}

/// The writers whose fds are open or that still write; flush_writers waits for them.
static WRITERS: Mutex<Vec<Arc<Writer>>> = Mutex::new(Vec::new());

impl Writer {
    fn new(handle: Handle) -> Arc<Writer> {
        let w = Arc::new(Writer { handle, st: Mutex::new(WriteState::default()), cv: Condvar::new() });
        lock(&WRITERS).push(w.clone());
        w
    }

    fn run(self: Arc<Self>) {
        loop {
            let mut st = lock(&self.st);
            while st.buf.is_empty() && !st.closing {
                st = self.cv.wait(st).unwrap_or_else(|e| e.into_inner());
            }
            if st.buf.is_empty() {
                break;
            }
            let chunk: Vec<u8> = st.buf.drain(..).collect();
            st.busy = true;
            drop(st);
            // the buffer has room again while the chunk is written
            self.cv.notify_all();
            notify();
            // in pieces, so that flush sees the thread get on while the reader takes them
            let mut r = Ok(());
            for piece in chunk.chunks(READ_CHUNK) {
                r = write_handle(self.handle, piece);
                if r.is_err() {
                    break;
                }
                lock(&self.st).written += 1;
                self.cv.notify_all();
            }
            let mut st = lock(&self.st);
            st.busy = false;
            if let Err(e) = r {
                st.err = Some(e.raw_os_error().unwrap_or(ERROR_BROKEN_PIPE));
                st.buf.clear();
            }
            drop(st);
            self.cv.notify_all();
            notify();
        }
        self.finish();
    }

    fn finish(&self) {
        unsafe { CloseHandle(self.handle) };
        lock(&self.st).done = true;
        self.cv.notify_all();
        let mut ws = lock(&WRITERS);
        if let Some(i) = ws.iter().position(|w| std::ptr::eq(&**w, self)) {
            ws.remove(i);
        }
    }

    fn writable(&self) -> bool {
        let st = lock(&self.st);
        st.err.is_some() || st.buf.len() < WRITE_BUFFER
    }

    fn write(self: &Arc<Self>, data: &[u8]) -> io::Result<usize> {
        let mut st = lock(&self.st);
        loop {
            if let Some(e) = st.err {
                return Err(io::Error::from_raw_os_error(e));
            }
            if st.buf.len() < WRITE_BUFFER || data.is_empty() {
                break;
            }
            st = self.cv.wait(st).unwrap_or_else(|e| e.into_inner());
        }
        // while the thread writes earlier bytes, the pipe may be full: then do not wait below
        let idle = st.buf.is_empty() && !st.busy;
        let n = data.len().min(WRITE_BUFFER - st.buf.len());
        st.buf.extend(&data[..n]);
        if !st.started {
            st.started = true;
            let w = self.clone();
            std::thread::spawn(move || w.run());
        }
        self.cv.notify_all();
        // Wait a little for the bytes to reach the pipe (they do at once while it has room), so
        // that a reader can read them now and a broken pipe is the error of this write.
        let until = Instant::now() + WRITE_THROUGH;
        while idle && (!st.buf.is_empty() || st.busy) && st.err.is_none() {
            let now = Instant::now();
            if now >= until {
                break;
            }
            st = self.cv.wait_timeout(st, until - now).unwrap_or_else(|e| e.into_inner()).0;
        }
        match st.err {
            Some(e) => Err(io::Error::from_raw_os_error(e)),
            None => Ok(n),
        }
    }

    /// Wait until what was written has reached the pipe (or failed), but not longer than limit,
    /// nor longer than stall without a piece written: the reader may be a VProc of this VM, which
    /// does not run while the VM's thread waits here, or a process that does not read.
    fn flush(&self, limit: Option<Duration>, stall: Duration) {
        let start = Instant::now();
        let mut st = lock(&self.st);
        let mut seen = (st.written, Instant::now());
        while (!st.buf.is_empty() || st.busy) && st.err.is_none() && !st.done {
            let now = Instant::now();
            if st.written != seen.0 {
                seen = (st.written, now);
            }
            let mut until = seen.1 + stall;
            if let Some(l) = limit {
                until = until.min(start + l);
            }
            if now >= until {
                break;
            }
            st = self.cv.wait_timeout(st, until - now).unwrap_or_else(|e| e.into_inner()).0;
        }
    }

    /// Close the fd: the thread writes out the buffer and closes the handle. Wait a little for
    /// it, as for a write, so that the reader mostly sees the end at once, as on Unix
    /// (flush_writers waits for the rest when the VM ends).
    fn close(self: &Arc<Self>) {
        let mut st = lock(&self.st);
        st.closing = true;
        if !st.started {
            st.started = true;
            drop(st);
            self.finish();
            return;
        }
        self.cv.notify_all();
        let until = Instant::now() + WRITE_THROUGH;
        while !st.done {
            let now = Instant::now();
            if now >= until {
                break;
            }
            st = self.cv.wait_timeout(st, until - now).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
}

fn write_handle(h: Handle, mut data: &[u8]) -> io::Result<()> {
    while !data.is_empty() {
        let mut n = 0u32;
        let len = data.len().min(u32::MAX as usize) as u32;
        if unsafe { WriteFile(h, data.as_ptr(), len, &mut n, std::ptr::null_mut()) } == 0 {
            return Err(last_error());
        }
        data = &data[n as usize..];
    }
    Ok(())
}

/// When the VM ends, let the writer threads write out what the VM wrote (the writes of a process
/// that exits on Unix are in the pipe already). First close the read ends that the VM has, so
/// that writes to a pipe that only a VProc of this VM read fail at once; then wait while each
/// thread gets on, but not for a reader that has stopped reading (a child that never reads:
/// on Unix the process ends and leaves the bytes in the pipe).
pub fn flush_writers() {
    let reads: Vec<Arc<Entry>> = {
        let mut t = table();
        let mut reads: Vec<Arc<Entry>> = Vec::new();
        for slot in t.slots.iter_mut() {
            if slot.as_ref().is_some_and(|e| matches!(**e, Entry::Pipe { owned: true, .. })) {
                let e = slot.take().unwrap();
                if !reads.iter().any(|r| Arc::ptr_eq(r, &e)) {
                    reads.push(e); // an fd and its dup share one entry: close it once
                }
            }
        }
        reads
    };
    for e in reads {
        e.close();
    }
    let ws: Vec<Arc<Writer>> = lock(&WRITERS).clone();
    for w in ws {
        w.flush(None, FLUSH_STALL);
    }
}

// ── Pipes in memory (the wakeup pipe) ───────────────────────────────────────

#[derive(Default)]
struct Mem {
    buf: Mutex<VecDeque<u8>>,
}

// ── The table of fds ────────────────────────────────────────────────────────

enum Entry {
    /// a file on disk (and a standard handle that is one): always ready
    File { handle: Handle, owned: bool },
    /// the read end of a pipe: ready when PeekNamedPipe finds bytes or the end
    Pipe { handle: Handle, owned: bool },
    /// standard output (1) or standard error (2) when it is not a file: written at once through
    /// std, as unix.rs writes fds 1 and 2, so that its order with write_std stays
    Std { fd: c_int, handle: Handle },
    Reader(Arc<Reader>),
    Writer(Arc<Writer>),
    MemRead(Arc<Mem>),
    MemWrite(Arc<Mem>),
}

impl Entry {
    fn close(&self) {
        match self {
            Entry::File { handle, owned: true } | Entry::Pipe { handle, owned: true } => unsafe {
                CloseHandle(*handle);
            },
            Entry::Reader(r) => r.close(),
            Entry::Writer(w) => w.close(),
            _ => {}
        }
    }

    fn readable(&self) -> bool {
        match self {
            Entry::Pipe { handle, .. } => peek(*handle),
            Entry::Reader(r) => r.ready(),
            Entry::MemRead(m) => !lock(&m.buf).is_empty(),
            _ => true,
        }
    }

    fn writable(&self) -> bool {
        match self {
            Entry::Writer(w) => w.writable(),
            _ => true,
        }
    }
}

/// The read end of a pipe has bytes, or is at its end (or has an error, which a read gives).
fn peek(h: Handle) -> bool {
    let mut avail = 0u32;
    let null = std::ptr::null_mut();
    if unsafe { PeekNamedPipe(h, null, 0, null.cast(), &mut avail, null.cast()) } == 0 {
        return true;
    }
    avail > 0
}

struct Table {
    slots: Vec<Option<Arc<Entry>>>,
}

static TABLE: OnceLock<Mutex<Table>> = OnceLock::new();

fn table() -> MutexGuard<'static, Table> {
    lock(TABLE.get_or_init(|| {
        // The standard handles that the VM got may be inheritable, and std starts every process
        // with the inheritable handles: then a child would hold the VM's standard output too,
        // and the reader of a $( ) would wait for it (on Unix, the child's fd 1 replaces it).
        // Stdio::inherit() makes its own inheritable copy, so the children that are to have
        // them still do.
        const HANDLE_FLAG_INHERIT: u32 = 1;
        for n in 0..3 {
            let h = unsafe { GetStdHandle((-10 - n) as u32) };
            if h != 0 && h != -1 {
                unsafe { SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0) };
            }
        }
        let slots = (0..3).map(std_entry).collect();
        Mutex::new(Table { slots })
    }))
}

/// The entry of fd 0, 1 or 2, from the standard handle (STD_INPUT_HANDLE is -10).
fn std_entry(fd: c_int) -> Option<Arc<Entry>> {
    let handle = unsafe { GetStdHandle((-10 - fd) as u32) };
    if handle == 0 || handle == -1 {
        return None;
    }
    let kind = unsafe { GetFileType(handle) };
    Some(Arc::new(if kind == FILE_TYPE_DISK {
        Entry::File { handle, owned: false }
    } else if fd == 0 && kind == FILE_TYPE_PIPE {
        Entry::Pipe { handle, owned: false }
    } else if fd == 0 {
        Entry::Reader(Reader::new(handle, false, kind == FILE_TYPE_CHAR))
    } else {
        Entry::Std { fd, handle }
    }))
}

impl Table {
    fn add(&mut self, e: Arc<Entry>) -> c_int {
        match self.slots.iter().position(Option::is_none) {
            Some(i) => {
                self.slots[i] = Some(e);
                i as c_int
            }
            None => {
                self.slots.push(Some(e));
                (self.slots.len() - 1) as c_int
            }
        }
    }

    fn get(&self, fd: c_int) -> io::Result<Arc<Entry>> {
        usize::try_from(fd).ok().and_then(|i| self.slots.get(i)).and_then(Clone::clone).ok_or_else(bad_fd)
    }
}

fn entry(fd: c_int) -> io::Result<Arc<Entry>> {
    table().get(fd)
}

fn add(e: Entry) -> c_int {
    table().add(Arc::new(e))
}

/// A pipe whose handles are not inherited by child processes, like os.pipe().
pub fn make_pipe() -> io::Result<(c_int, c_int)> {
    let (mut r, mut w) = (0, 0);
    if unsafe { CreatePipe(&mut r, &mut w, std::ptr::null(), WRITE_BUFFER as u32) } == 0 {
        return Err(last_error());
    }
    Ok((add(Entry::Pipe { handle: r, owned: true }), add(Entry::Writer(Writer::new(w)))))
}

/// Another fd for the same pipe or file, like os.dup().
pub fn dup(fd: c_int) -> io::Result<c_int> {
    let mut t = table();
    let e = t.get(fd)?;
    Ok(t.add(e))
}

pub fn close_fd(fd: c_int) -> io::Result<()> {
    let e = {
        let mut t = table();
        let e = t.get(fd)?;
        t.slots[fd as usize] = None;
        if t.slots.iter().flatten().any(|o| Arc::ptr_eq(o, &e)) {
            return Ok(()); // another fd still has it
        }
        e
    };
    e.close();
    Ok(())
}

pub fn read_fd(fd: c_int, buf: &mut [u8]) -> io::Result<usize> {
    match &*entry(fd)? {
        // a pipe that has no bytes blocks, as read() does on Unix
        Entry::File { handle, .. } | Entry::Pipe { handle, .. } => {
            let mut n = 0u32;
            let len = buf.len().min(u32::MAX as usize) as u32;
            if unsafe { ReadFile(*handle, buf.as_mut_ptr(), len, &mut n, std::ptr::null_mut()) } == 0 {
                let e = last_error();
                return match e.raw_os_error() {
                    Some(ERROR_BROKEN_PIPE) | Some(ERROR_HANDLE_EOF) => Ok(0),
                    _ => Err(e),
                };
            }
            Ok(n as usize)
        }
        Entry::Reader(r) => r.read(buf),
        Entry::MemRead(m) => {
            let mut b = lock(&m.buf);
            let n = buf.len().min(b.len());
            for (o, x) in buf.iter_mut().zip(b.drain(..n)) {
                *o = x;
            }
            Ok(n)
        }
        _ => Err(bad_fd()),
    }
}

pub fn write_fd(fd: c_int, buf: &[u8]) -> io::Result<usize> {
    match &*entry(fd)? {
        Entry::File { handle, .. } => write_handle(*handle, buf).map(|_| buf.len()),
        Entry::Std { fd, .. } => {
            use std::io::Write;
            if *fd == 2 {
                let mut e = std::io::stderr().lock();
                e.write_all(buf).and_then(|_| e.flush())?;
            } else {
                let mut o = std::io::stdout().lock();
                o.write_all(buf).and_then(|_| o.flush())?;
            }
            Ok(buf.len())
        }
        Entry::Writer(w) => w.write(buf),
        Entry::MemWrite(m) => {
            lock(&m.buf).extend(buf);
            notify();
            Ok(buf.len())
        }
        _ => Err(bad_fd()),
    }
}

/// Wait until some of the fds are ready, as select(): reads are the fds to read, writes the fds
/// to write; timeout None blocks. Returns the ready fds (an fd that is not open is ready, as
/// poll() gives POLLNVAL).
pub fn wait_fds(reads: &[c_int], writes: &[c_int], timeout: Option<i32>) -> io::Result<Vec<c_int>> {
    let deadline = timeout.map(|ms| Instant::now() + Duration::from_millis(ms.max(0) as u64));
    loop {
        let seen = *lock(&CHANGES);
        let mut ready = Vec::new();
        for &fd in reads {
            if entry(fd).map_or(true, |e| e.readable()) {
                ready.push(fd);
            }
        }
        for &fd in writes {
            if entry(fd).map_or(true, |e| e.writable()) {
                ready.push(fd);
            }
        }
        if !ready.is_empty() {
            return Ok(ready);
        }
        // nothing tells when a pipe gets bytes: look again soon
        let peeks = reads.iter().any(|&fd| entry(fd).is_ok_and(|e| matches!(*e, Entry::Pipe { .. })));
        let mut g = lock(&CHANGES);
        while *g == seen {
            let now = Instant::now();
            if deadline.is_some_and(|d| now >= d) {
                return Ok(ready);
            }
            let mut until = deadline;
            if peeks {
                until = Some(until.map_or(now + PEEK_EVERY, |d| d.min(now + PEEK_EVERY)));
            }
            match until {
                None => g = COND.wait(g).unwrap_or_else(|e| e.into_inner()),
                Some(u) => {
                    g = COND.wait_timeout(g, u.saturating_duration_since(now)).unwrap_or_else(|e| e.into_inner()).0;
                    if peeks && Instant::now() >= u {
                        break;
                    }
                }
            }
        }
    }
}

/// True if fd is ready to read (or at EOF) without blocking.
pub fn readable(fd: c_int) -> io::Result<bool> {
    Ok(entry(fd)?.readable())
}

/// True if fd can be written without blocking.
pub fn writable(fd: c_int) -> io::Result<bool> {
    Ok(entry(fd)?.writable())
}

// ── Processes ───────────────────────────────────────────────────────────────

/// The standard stream of a child process from fd: a copy of its handle, after a read in
/// progress has been stopped (so that what the child is to read does not go to the reader
/// thread) and what the VM wrote has reached the pipe.
pub fn child_stdio(fd: c_int) -> io::Result<Stdio> {
    let handle = match &*entry(fd)? {
        Entry::File { handle, .. } | Entry::Std { handle, .. } | Entry::Pipe { handle, .. } => *handle,
        Entry::Reader(r) => {
            r.cancel(false);
            r.handle
        }
        Entry::Writer(w) => {
            // not longer than a write waits: the reader may be a VProc of this VM, which runs
            // only after this (the bytes left then reach the pipe after the child's may)
            w.flush(Some(WRITE_THROUGH), WRITE_THROUGH);
            w.handle
        }
        _ => return Err(bad_fd()),
    };
    let mut copy = 0;
    let me = unsafe { GetCurrentProcess() };
    if unsafe { DuplicateHandle(me, handle, me, &mut copy, 0, 0, DUPLICATE_SAME_ACCESS) } == 0 {
        return Err(last_error());
    }
    Ok(Stdio::from(unsafe { OwnedHandle::from_raw_handle(copy as *mut c_void) }))
}

/// The standard stream n (0, 1 or 2) of a child process that gets the VM's own. Standard input
/// goes to the child as it is, so stop a console read that a VProc started on fd 0, as
/// child_stdio does: else the reader thread and the child take the user's lines in turn.
pub fn inherited_stdio(n: usize) -> Stdio {
    if n == 0 {
        if let Ok(e) = entry(0) {
            if let Entry::Reader(r) = &*e {
                r.cancel(false);
            }
        }
    }
    Stdio::inherit()
}

/// The arguments of the process after its name. Rust leaves an argument without spaces or
/// quotes unquoted, and the programs of MSYS (Git Bash) take a backslash outside quotes as an
/// escape and drop it (printf 'a\nb' gave "anb"), expand * ? [ ] { } as a glob (touch 'x[1]'
/// made x1), take ' as a quote, and expand a ~ at the start. An argument with one of them goes
/// in double quotes, as the C runtime of Windows reads them: backslashes are as they are,
/// except those before a quote, which are doubled, and the quote gets one more. MSYS reads
/// "\\" in quotes as one backslash, which no form of the argument gives to both runtimes:
/// a ConMa argument with two backslashes in a row reaches an MSYS program with one (a known
/// difference of Windows in docs/dev/TESTS-ja.md).
pub fn add_args(cmd: &mut Command, args: &[String]) {
    use std::os::windows::process::CommandExt;
    for a in args {
        if !a.contains(['\\', '*', '?', '[', ']', '{', '}', '\'', '~']) {
            cmd.arg(a);
            continue;
        }
        let mut q = String::from("\"");
        let mut backslashes = 0;
        for c in a.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    q.extend(std::iter::repeat('\\').take(backslashes * 2 + 1));
                    backslashes = 0;
                }
                _ => {
                    q.extend(std::iter::repeat('\\').take(backslashes));
                    backslashes = 0;
                }
            }
            if c != '\\' {
                q.push(c);
            }
        }
        q.extend(std::iter::repeat('\\').take(backslashes * 2));
        q.push('"');
        cmd.raw_arg(q);
    }
}

/// Start the process apart from the console's Ctrl-C (__OS_spawn_detached__; setsid on Unix).
pub fn detach(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x200;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// Wake the scheduler when the child exits, as SIGCHLD does on Unix.
pub fn watch_process(child: &Child) {
    let mut h = 0;
    let me = unsafe { GetCurrentProcess() };
    if unsafe { DuplicateHandle(me, child.as_raw_handle() as Handle, me, &mut h, 0, 0, DUPLICATE_SAME_ACCESS) } == 0 {
        return;
    }
    std::thread::spawn(move || {
        unsafe {
            WaitForSingleObject(h, INFINITE);
            CloseHandle(h);
        }
        on_signal();
    });
}

/// __OS_kill__: Windows has no signals; end the process, with an exit code that exit_code turns
/// into the negative signal number, as on Unix.
pub fn kill_process(child: &Child, sig: c_int) -> io::Result<()> {
    if unsafe { TerminateProcess(child.as_raw_handle() as Handle, KILLED + sig as u32) } == 0 {
        return Err(last_error());
    }
    Ok(())
}

/// The exit code, or a negative signal number for a process that __OS_kill__ or Ctrl-C ended.
pub fn exit_code(st: ExitStatus) -> i32 {
    let c = st.code().unwrap_or(0) as u32;
    if c & !0xFF == KILLED {
        -((c & 0xFF) as i32)
    } else if c == CONTROL_C_EXIT {
        -SIGINT
    } else {
        c as i32
    }
}

pub fn can_search(path: &str) -> bool {
    is_dir(path).unwrap_or(false)
}

/// The values of the names of sysconf that Windows can answer: the page size and the number of
/// processors (GetSystemInfo), the longest command line (32,767 characters, CreateProcess) and
/// the most files the C runtime can open (8,192, _setmaxstdio).
pub fn sysconf_value(name: c_int) -> io::Result<c_long> {
    let mut info: SystemInfo = unsafe { std::mem::zeroed() };
    unsafe { GetSystemInfo(&mut info) };
    Ok(match name {
        1 => info.page_size as c_long,
        2 => info.number_of_processors as c_long,
        3 => 32767,
        _ => 8192,
    })
}

pub fn sysconf_name(name: &str) -> Option<c_int> {
    let table: &[(&str, c_int)] = &[
        ("SC_PAGESIZE", 1),
        ("SC_PAGE_SIZE", 1),
        ("SC_NPROCESSORS_CONF", 2),
        ("SC_NPROCESSORS_ONLN", 2),
        ("SC_ARG_MAX", 3),
        ("SC_OPEN_MAX", 4),
    ];
    table.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
}

// ── Files, the environment and standard output ──────────────────────────────

/// Open path with mode "r", "w" (create, truncate) or "a" (create, append); a directory is
/// EISDIR. Returns the fd.
pub fn open_file(path: &str, mode: &str) -> io::Result<c_int> {
    // The devices of Unix that ConSh uses (`> /dev/null`, `> /dev/stdout`): /dev/null is the device
    // NUL of Windows, and /dev/stdin, stdout and stderr are copies of fds 0, 1 and 2. The VM joins
    // the path to the VProc's directory, which gives it a drive (C:/dev/null)
    let unix = if path.starts_with('/') { Some(path) } else { path.get(1..).filter(|p| p.starts_with(":/")).map(|p| &p[1..]) };
    match unix {
        Some("/dev/stdin") => return dup(0),
        Some("/dev/stdout") => return dup(1),
        Some("/dev/stderr") => return dup(2),
        _ => {}
    }
    let path = if unix == Some("/dev/null") { "NUL" } else { path };
    if path != "NUL" {
        check_on_the_way(path)?;
    }
    if is_dir(path).unwrap_or(false) {
        return Err(errno_error(21)); // EISDIR
    }
    let mut opts = std::fs::OpenOptions::new();
    match mode {
        "r" => opts.read(true),
        "w" => opts.write(true).create(true).truncate(true),
        _ => opts.append(true).create(true),
    };
    let f = opts.open(path)?;
    Ok(add(Entry::File { handle: f.into_raw_handle() as Handle, owned: true }))
}

/// Whether path is a directory; an error if it does not exist.
pub fn is_dir(path: &str) -> io::Result<bool> {
    check_on_the_way(path)?;
    Ok(std::fs::metadata(path)?.is_dir())
}

/// The errors of Unix for the names on the way to the last one of path: ENOENT if one does not
/// exist, ENOTDIR if one is a file (Windows says ERROR_PATH_NOT_FOUND for both, and goes over
/// "name/.." without looking at name).
fn check_on_the_way(path: &str) -> io::Result<()> {
    use std::path::Component;
    let mut on_the_way = std::path::PathBuf::new();
    let components: Vec<Component> = std::path::Path::new(path).components().collect();
    for (i, c) in components.iter().enumerate() {
        if i + 1 < components.len() && matches!(c, Component::Normal(_)) {
            on_the_way.push(c);
            match std::fs::metadata(&on_the_way) {
                Ok(m) if !m.is_dir() => return Err(errno_error(20)), // ENOTDIR
                Err(_) => return Err(errno_error(2)),                // ENOENT
                Ok(_) => {}
            }
        } else {
            on_the_way.push(c);
        }
    }
    Ok(())
}

/// A path of Windows as the VM writes it: C:/Users/..., without the \\?\ of canonicalize.
pub fn path_text(p: &std::path::Path) -> String {
    let s = p.to_string_lossy();
    // \\?\UNC\srv\share (from canonicalize) is //srv/share
    if let Some(unc) = s.strip_prefix(r"\\?\UNC\") {
        return format!("//{}", unc.replace('\\', "/"));
    }
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

/// The absolute path without symbolic links, . and ..
pub fn canonicalize(path: &str) -> io::Result<String> {
    check_on_the_way(path)?;
    Ok(path_text(&std::fs::canonicalize(path)?))
}

/// The names in the directory path (in no particular order).
pub fn read_dir(path: &str) -> io::Result<Vec<String>> {
    std::fs::read_dir(path)?.map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned())).collect()
}

/// The environment variables of the interpreter.
pub fn env_vars() -> Vec<(String, String)> {
    std::env::vars_os().map(|(k, v)| (k.to_string_lossy().into_owned(), v.to_string_lossy().into_owned())).collect()
}

pub fn getpid() -> u32 {
    std::process::id()
}

/// __JS_call__: the native VM has no JavaScript (unix.rs).
pub fn js_call(_code: &str, _args_json: &str) -> io::Result<(c_int, c_int)> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "JavaScript is not available"))
}

/// __Host_call__: the native VM has no host (unix.rs).
pub fn host_call(_name: &str, _args_json: &str) -> Option<Result<String, String>> {
    None
}

pub fn host_has(_name: &str) -> bool {
    false
}

pub fn host_start(_name: &str, _args_json: &str, w: c_int) {
    let _ = close_fd(w);
}

pub fn host_feature(_name: &str) -> bool {
    false
}

pub fn js_available() -> bool {
    false
}

pub fn js_value(_id: &str) -> io::Result<String> {
    Err(io::Error::new(io::ErrorKind::Unsupported, "JavaScript is not available"))
}

pub fn wasm_compile(_bytes: &[u8]) -> Result<i32, String> {
    Err("WebAssembly is not available".into())
}

pub fn wasm_call(_host: i32, _name: &str, _args: &[f64], _memory: &mut [f64], _fuel: f64) -> WasmResult {
    WasmResult::Trap("WebAssembly is not available".into())
}

pub fn wasm_free(_host: i32) {}

pub fn wasm_available() -> bool {
    false
}

/// How a call of __Wasm_call__ ended.
#[allow(dead_code)]
pub enum WasmResult {
    Ok(f64),
    Fuel,
    Trap(String),
}

#[allow(dead_code)]
pub fn unsupported() -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, "not supported")
}

/// Write text to fd 1 or 2 at once, so that the order of standard output and standard error is
/// kept when both go to the same file.
pub fn write_std(fd: c_int, text: &str) {
    use std::io::Write;
    let r = if fd == 2 {
        let mut e = std::io::stderr().lock();
        e.write_all(text.as_bytes()).and_then(|_| e.flush())
    } else {
        let mut o = std::io::stdout().lock();
        o.write_all(text.as_bytes()).and_then(|_| o.flush())
    };
    // Like a process killed by SIGPIPE on Unix: stop when the reader has gone.
    if let Err(e) = r {
        if e.kind() == io::ErrorKind::BrokenPipe {
            flush_writers();
            std::process::exit(141);
        }
    }
}

// ── Signals ─────────────────────────────────────────────────────────────────

/// Write end of the pipe that the exits of children and Ctrl-C write to, so that a wait for fds
/// wakes up.
static WAKE_FD: AtomicI32 = AtomicI32::new(-1);
/// The number of Ctrl-Cs since __OS_trap_interrupt__.
static INTERRUPTS: AtomicU64 = AtomicU64::new(0);
/// The number of exits of children and Ctrl-Cs that wrote to the wakeup pipe.
static SIGNALS: AtomicU64 = AtomicU64::new(0);

fn on_signal() {
    SIGNALS.fetch_add(1, Ordering::SeqCst);
    let fd = WAKE_FD.load(Ordering::Relaxed);
    if fd >= 0 {
        let _ = write_fd(fd, b"x");
    }
    notify();
}

unsafe extern "system" fn on_ctrl(kind: u32) -> Bool {
    if kind > CTRL_BREAK_EVENT {
        return 0;
    }
    INTERRUPTS.fetch_add(1, Ordering::SeqCst);
    on_signal();
    1
}

/// A pipe in memory: (read end, write end). Writing to it wakes wait_fds; reading never blocks.
pub fn make_wake_pipe() -> io::Result<(c_int, c_int)> {
    let m = Arc::new(Mem::default());
    Ok((add(Entry::MemRead(m.clone())), add(Entry::MemWrite(m))))
}

pub fn install_wakeup() -> io::Result<c_int> {
    let (r, w) = make_wake_pipe()?;
    WAKE_FD.store(w, Ordering::SeqCst);
    Ok(r)
}

/// Count Ctrl-Cs instead of letting them stop the VM.
pub fn trap_sigint() {
    unsafe { SetConsoleCtrlHandler(Some(on_ctrl), 1) };
}

pub fn interrupts() -> u64 {
    INTERRUPTS.load(Ordering::SeqCst)
}

/// True if a Ctrl-C has been counted since the count was before, waiting up to CTRL_C_WAIT for one.
fn ctrl_c_since(before: u64) -> bool {
    let until = Instant::now() + CTRL_C_WAIT;
    loop {
        if INTERRUPTS.load(Ordering::SeqCst) != before {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(PEEK_EVERY);
    }
}

/// The number of exits of children and Ctrl-Cs so far.
pub fn signals() -> u64 {
    SIGNALS.load(Ordering::SeqCst)
}

/// Read and drop what was written to the wakeup pipe.
pub fn drain(fd: c_int) {
    if let Ok(e) = entry(fd) {
        if let Entry::MemRead(m) = &*e {
            lock(&m.buf).clear();
        }
    }
}

// ── Error messages as Python writes them on macOS and Linux ─────────────────

/// Errors that are an errno with no error of Windows (EISDIR): ERRNO + the errno.
const ERRNO: i32 = 0x2000_0000;

pub fn errno_error(n: i32) -> io::Error {
    io::Error::from_raw_os_error(ERRNO + n)
}

/// The errno and its text for the errors of Windows that the tests meet.
fn errno_of(e: &io::Error) -> Option<(i32, &'static str)> {
    // Command::spawn gives "program not found", with no number, for a program not in PATH
    let raw = match e.raw_os_error() {
        Some(n) => n,
        None if e.kind() == io::ErrorKind::NotFound => 2,
        None => return None,
    };
    let n = match raw {
        2 | 3 => 2,
        5 => 13,
        6 => 9,
        80 | 183 => 17,
        109 | 232 => 32,
        267 => 20,
        n if n > ERRNO => n - ERRNO,
        _ => return None,
    };
    let text = match n {
        2 => "No such file or directory",
        9 => "Bad file descriptor",
        13 => "Permission denied",
        17 => "File exists",
        20 => "Not a directory",
        21 => "Is a directory",
        32 => "Broken pipe",
        _ => "Unknown error",
    };
    Some((n, text))
}

/// e.strerror: the text of the C strerror.
pub fn strerror(e: &io::Error) -> String {
    if let Some((_, t)) = errno_of(e) {
        return t.to_string();
    }
    let s = e.to_string();
    match s.find(" (os error") {
        Some(i) => s[..i].to_string(),
        None => s,
    }
}

/// str(e) of an OSError without a file name: "[Errno 9] Bad file descriptor", or
/// "[WinError 1] Incorrect function." for an error of Windows with no errno.
pub fn errno_str(e: &io::Error) -> String {
    match (errno_of(e), e.raw_os_error()) {
        (Some((n, t)), _) => format!("[Errno {}] {}", n, t),
        (None, Some(n)) => format!("[WinError {}] {}", n, strerror(e)),
        _ => e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests share the table of fds: one at a time.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn read_all(fd: c_int) -> Vec<u8> {
        let mut out = Vec::new();
        let mut buf = [0u8; 1000];
        loop {
            let n = read_fd(fd, &mut buf).unwrap();
            if n == 0 {
                return out;
            }
            out.extend(&buf[..n]);
        }
    }

    #[test]
    fn fds_are_the_lowest_free_numbers() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        assert!(r >= 3 && w == r + 1);
        close_fd(r).unwrap();
        let d = dup(w).unwrap();
        assert_eq!(d, r);
        close_fd(d).unwrap();
        close_fd(w).unwrap();
        assert_eq!(errno_str(&close_fd(w).unwrap_err()), "[Errno 9] Bad file descriptor");
    }

    #[test]
    fn a_pipe_carries_what_is_written_and_ends_when_closed() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        assert!(writable(w).unwrap());
        assert_eq!(wait_fds(&[r], &[], Some(0)).unwrap(), Vec::<c_int>::new());
        write_fd(w, &vec![b'a'; 100_000]).and_then(|n| write_fd(w, &vec![b'a'; 100_000 - n])).unwrap();
        // the buffer is full until the reader reads
        let reader = std::thread::spawn(move || read_all(r));
        close_fd(w).unwrap();
        assert_eq!(reader.join().unwrap().len(), 100_000);
        close_fd(r).unwrap();
    }

    #[test]
    fn what_is_written_can_be_read_at_once() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        assert!(!readable(r).unwrap());
        write_fd(w, b"hello").unwrap();
        assert!(readable(r).unwrap());
        assert_eq!(wait_fds(&[r], &[w], Some(0)).unwrap(), vec![r, w]);
        let mut buf = [0u8; 10];
        assert_eq!(read_fd(r, &mut buf).unwrap(), 5);
        close_fd(w).unwrap();
        assert!(readable(r).unwrap()); // the end
        assert_eq!(read_fd(r, &mut buf).unwrap(), 0);
        close_fd(r).unwrap();
    }

    #[test]
    fn wait_fds_finds_bytes_that_come_later() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            write_fd(w, b"x").unwrap();
            w
        });
        assert_eq!(wait_fds(&[r], &[], None).unwrap(), vec![r]);
        close_fd(t.join().unwrap()).unwrap();
        close_fd(r).unwrap();
    }

    #[test]
    fn nothing_is_read_ahead_of_the_vm() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        assert!(!readable(r).unwrap());
        write_fd(w, b"for the child").unwrap();
        let mut buf = [0u8; 100];
        assert_eq!(read_fd(r, &mut buf[..4]).unwrap(), 4);
        let _stdio = child_stdio(r).unwrap();
        let Entry::Pipe { handle, .. } = &*entry(r).unwrap() else { panic!() };
        let mut n = 0u32;
        // the rest is still in the pipe for the child
        assert!(unsafe { ReadFile(*handle, buf.as_mut_ptr(), 100, &mut n, std::ptr::null_mut()) } != 0);
        assert_eq!(&buf[..n as usize], b"the child");
        close_fd(w).unwrap();
        close_fd(r).unwrap();
    }

    #[test]
    fn flush_stops_waiting_for_a_reader_that_does_not_read() {
        // more than the pipe and the writer's buffer hold: the writer thread stays in WriteFile
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        let data = vec![b'x'; WRITE_BUFFER];
        let mut sent = 0;
        while sent < 3 * WRITE_BUFFER {
            sent += write_fd(w, &data).unwrap();
        }
        let start = Instant::now();
        // the limit of child_stdio
        if let Entry::Writer(wr) = &*entry(w).unwrap() {
            wr.flush(Some(WRITE_THROUGH), WRITE_THROUGH);
            assert!(start.elapsed() < Duration::from_secs(1));
            // the stall of flush_writers, while the reader reads a piece now and then
            let start = Instant::now();
            let reader = std::thread::spawn(move || {
                let mut buf = vec![0u8; READ_CHUNK];
                for _ in 0..5 {
                    std::thread::sleep(Duration::from_millis(50));
                    read_fd(r, &mut buf).unwrap();
                }
                r
            });
            wr.flush(None, Duration::from_millis(150));
            let r = reader.join().unwrap();
            let t = start.elapsed();
            assert!(t >= Duration::from_millis(250) && t < Duration::from_secs(2), "{t:?}");
            close_fd(r).unwrap();
        } else {
            panic!("not a writer");
        }
        close_fd(w).unwrap();
    }

    #[test]
    fn writing_to_a_pipe_without_a_reader_is_epipe() {
        let _g = lock(&SERIAL);
        let (r, w) = make_pipe().unwrap();
        close_fd(r).unwrap();
        assert!(writable(w).unwrap());
        assert_eq!(errno_str(&write_fd(w, b"x").unwrap_err()), "[Errno 32] Broken pipe");
        close_fd(w).unwrap();
    }

    #[test]
    fn the_wakeup_pipe_wakes_wait_fds() {
        let _g = lock(&SERIAL);
        let (r, w) = make_wake_pipe().unwrap();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            write_fd(w, b"x").unwrap();
        });
        assert_eq!(wait_fds(&[r], &[], None).unwrap(), vec![r]);
        drain(r);
        assert!(!readable(r).unwrap());
        t.join().unwrap();
    }

    #[test]
    fn errors_are_written_as_python_writes_them_on_unix() {
        let e = |n| io::Error::from_raw_os_error(n);
        assert_eq!(strerror(&e(2)), "No such file or directory");
        assert_eq!(errno_str(&e(3)), "[Errno 2] No such file or directory");
        assert_eq!(errno_str(&e(5)), "[Errno 13] Permission denied");
        assert_eq!(errno_str(&e(232)), "[Errno 32] Broken pipe");
        assert_eq!(errno_str(&e(267)), "[Errno 20] Not a directory");
        assert_eq!(strerror(&errno_error(21)), "Is a directory");
        assert!(errno_str(&e(1)).starts_with("[WinError 1] "));
        assert_eq!(strerror(&std::fs::File::open("no such file").unwrap_err()), "No such file or directory");
    }

    #[test]
    fn paths_are_written_with_slashes() {
        assert_eq!(path_text(std::path::Path::new(r"\\?\C:\Users\a b\conma")), "C:/Users/a b/conma");
        assert_eq!(path_text(&std::path::Path::new("C:/x").join("y")), "C:/x/y");
        assert_eq!(path_text(std::path::Path::new(r"\\?\UNC\srv\share\a")), "//srv/share/a");
        let dir = std::env::temp_dir();
        assert!(!canonicalize(&dir.to_string_lossy()).unwrap().contains('\\'));
        let p = path_text(&dir);
        assert_eq!(errno_str(&open_file(&p, "r").unwrap_err()), "[Errno 21] Is a directory");
    }

    #[test]
    fn kill_gives_the_exit_code_of_the_signal() {
        let mut child = Command::new("cmd").args(["/c", "pause"]).stdin(Stdio::piped()).stdout(Stdio::null()).spawn().unwrap();
        kill_process(&child, 15).unwrap();
        assert_eq!(exit_code(child.wait().unwrap()), -15);
        let ok = Command::new("cmd").args(["/c", "exit 3"]).status().unwrap();
        assert_eq!(exit_code(ok), 3);
    }
}
