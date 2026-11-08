//! OS Stream, file and directory primitives.

use super::{take_n, PResult, VmError};
use crate::sched::{Machine, OsTarget};
use crate::sys;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::rc::Rc;

pub fn new_stream(m: &mut Machine, fd: i32, mode: char) -> Rc<StreamObj> {
    m.serials.streams += 1;
    Rc::new(StreamObj::new(m.serials.streams, fd, mode, false))
}

fn std_stream(m: &mut Machine, fd: usize, mode: char) -> Value {
    if m.std_streams[fd].is_none() {
        m.serials.streams += 1;
        m.std_streams[fd] = Some(Rc::new(StreamObj::new(m.serials.streams, fd as i32, mode, true)));
    }
    Value::Stream(m.std_streams[fd].clone().unwrap())
}

pub fn get_stdin(m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = std_stream(m, 0, 'r');
    vp.prim_return(vec![s]);
    Ok(())
}
pub fn get_stdout(m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = std_stream(m, 1, 'w');
    vp.prim_return(vec![s]);
    Ok(())
}
pub fn get_stderr(m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = std_stream(m, 2, 'w');
    vp.prim_return(vec![s]);
    Ok(())
}

/// An open Stream of the given mode ('r', 'w' or any); onError and None if not.
pub fn check_stream(vp: &mut VProc, on_error: &Value, name: &str, v: &Value, mode: Option<char>) -> Option<Rc<StreamObj>> {
    let s = match v {
        Value::Stream(s) if mode.map_or(true, |m| m == s.mode) => s.clone(),
        _ => {
            let expected = match mode {
                Some('r') => "input Stream",
                Some(_) => "output Stream",
                None => "Stream",
            };
            vp.prim_error(on_error.clone(), &format!("{}: expected {}, got {}", name, expected, value_type_name(v)));
            return None;
        }
    };
    if s.closed.get() {
        vp.prim_error(on_error.clone(), &format!("{}: closed {}", name, s.kind()));
        return None;
    }
    Some(s)
}

/// onError and the Stream argument.
fn stream_args(vp: &mut VProc, name: &str, mode: Option<char>) -> Result<Option<(Value, Rc<StreamObj>)>, VmError> {
    let mut a = take_n(vp, name, 2)?;
    let v = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    Ok(check_stream(vp, &on_error, name, &v, mode).map(|s| (on_error, s)))
}

pub fn read(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_read__";
    let (on_error, s) = match stream_args(vp, name, Some('r'))? {
        Some(x) => x,
        None => return Ok(()),
    };
    // Check for data with poll instead of setting O_NONBLOCK: the flag
    // belongs to the open pipe, which a child process may share.
    let mut buf = [0u8; 4096];
    let r = sys::readable(s.fd).and_then(|ok| if ok { sys::read_fd(s.fd, &mut buf).map(Some) } else { Ok(None) });
    match r {
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::errno_str(&e))),
        Ok(None) => vp.prim_return(vec![string_value("")]),
        Ok(Some(0)) => {
            let rest = s.decode(b"", true);
            vp.prim_return(vec![if rest.is_empty() { Value::Null } else { string_value(&rest) }]);
        }
        Ok(Some(n)) => {
            let text = s.decode(&buf[..n], false);
            vp.prim_return(vec![string_value(&text)]);
        }
    }
    Ok(())
}

pub fn write(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_write__";
    let mut a = take_n(vp, name, 3)?;
    let val = a.pop().unwrap();
    let v = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let s = match check_stream(vp, &on_error, name, &v, Some('w')) {
        Some(s) => s,
        None => return Ok(()),
    };
    let text = value_to_display(&val).map_err(VmError::Error)?;
    let mut data = text.as_bytes();
    while !data.is_empty() {
        match sys::write_fd(s.fd, data) {
            Ok(n) => data = &data[n..],
            Err(e) => {
                vp.prim_error(on_error, &format!("{}: {}", name, sys::errno_str(&e)));
                return Ok(());
            }
        }
    }
    vp.prim_return(vec![]);
    Ok(())
}

pub fn close(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_close__";
    let (on_error, s) = match stream_args(vp, name, None)? {
        Some(x) => x,
        None => return Ok(()),
    };
    match s.close() {
        Ok(()) => vp.prim_return(vec![]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::errno_str(&e))),
    }
    Ok(())
}

pub fn dup(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_dup__";
    let (on_error, s) = match stream_args(vp, name, None)? {
        Some(x) => x,
        None => return Ok(()),
    };
    match sys::dup(s.fd) {
        Ok(fd) => {
            let d = new_stream(m, fd, s.mode);
            vp.prim_return(vec![Value::Stream(d)]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::strerror(&e))),
    }
    Ok(())
}

pub fn pipe(m: &mut Machine, vp: &mut VProc) -> PResult {
    let (r, w) = sys::make_pipe().map_err(|e| VmError::Error(format!("__OS_pipe__: {}", sys::errno_str(&e))))?;
    let rs = new_stream(m, r, 'r');
    let ws = new_stream(m, w, 'w');
    vp.prim_return(vec![Value::Stream(rs), Value::Stream(ws)]);
    Ok(())
}

fn wait_ready(m: &mut Machine, vp: &mut VProc, name: &str, mode: char) -> PResult {
    let s = match stream_args(vp, name, Some(mode))? {
        Some((_, s)) => s,
        None => return Ok(()),
    };
    let ready = if mode == 'r' { sys::readable(s.fd) } else { sys::writable(s.fd) }.unwrap_or(false);
    vp.prim_return(vec![]);
    if !ready {
        let cur = m.current.clone().expect("no current VProc");
        m.wait_os(&cur, OsTarget::Stream(s));
    }
    Ok(())
}

pub fn wait_readable(m: &mut Machine, vp: &mut VProc) -> PResult {
    wait_ready(m, vp, "__OS_wait_readable__", 'r')
}
pub fn wait_writable(m: &mut Machine, vp: &mut VProc) -> PResult {
    wait_ready(m, vp, "__OS_wait_writable__", 'w')
}

/// __OS_get_String__ ,(String): one line of standard input, or null at EOF.
pub fn get_string(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut line = Vec::new();
    let mut b = [0u8; 1];
    while let Ok(1) = sys::read_fd(0, &mut b) {
        line.push(b[0]);
        if b[0] == b'\n' {
            break;
        }
    }
    let r = if line.is_empty() { Value::Null } else { string_value(&String::from_utf8_lossy(&line)) };
    vp.prim_return(vec![r]);
    Ok(())
}

// ── Files and directories ───────────────────────────────────────────────────

/// path relative to the current directory of the VProc; "" stays "".
fn vproc_path(vp: &VProc, path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        std::path::Path::new(&vp.cwd).join(path).to_string_lossy().into_owned()
    }
}

/// onError and one String argument, as text.
fn path_args(vp: &mut VProc, name: &str, what: &str) -> Result<Option<(Value, String)>, VmError> {
    let mut a = take_n(vp, name, 2)?;
    let path = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    if value_type_name(&path) != "String" {
        vp.prim_error(on_error, &format!("{}: expected String {}, got {}", name, what, value_type_name(&path)));
        return Ok(None);
    }
    let text = value_to_display(&path).map_err(VmError::Error)?;
    Ok(Some((on_error, text)))
}

fn io_error(errno: i32) -> std::io::Error {
    std::io::Error::from_raw_os_error(errno)
}

pub fn open(m: &mut Machine, vp: &mut VProc) -> PResult {
    open_with(m, vp, "__OS_open__", false)
}

/// __OS_open_tracked__ onError path mode Ref ,(Stream): __OS_open__, and in
/// the same step, put the Stream in front of the LList in the Ref, so that a
/// VProc killed between two steps never leaves a file open but not recorded.
pub fn open_tracked(m: &mut Machine, vp: &mut VProc) -> PResult {
    open_with(m, vp, "__OS_open_tracked__", true)
}

fn open_with(m: &mut Machine, vp: &mut VProc, name: &str, tracked: bool) -> PResult {
    let mut a = take_n(vp, name, if tracked { 4 } else { 3 })?;
    let track = if tracked { a.pop() } else { None };
    let mode = a.pop().unwrap();
    let path = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let track = match track {
        None => None,
        Some(Value::Ref(r)) if matches!(&*r.value.borrow(), Value::Null | Value::LList(_)) => Some(r),
        Some(v) => {
            let got = match &v {
                Value::Ref(r) => format!("a Ref of {}", value_type_name(&r.value.borrow())),
                other => value_type_name(other).to_string(),
            };
            vp.prim_error(on_error, &format!("{}: expected a Ref of an LList, got {}", name, got));
            return Ok(());
        }
    };
    for (what, v) in [("path", &path), ("mode", &mode)] {
        if value_type_name(v) != "String" {
            vp.prim_error(on_error, &format!("{}: expected String {}, got {}", name, what, value_type_name(v)));
            return Ok(());
        }
    }
    let path = value_to_display(&path).map_err(VmError::Error)?;
    let mode = value_to_display(&mode).map_err(VmError::Error)?;
    let smode = match mode.as_str() {
        "r" => 'r',
        "w" | "a" => 'w',
        _ => {
            vp.prim_error(on_error, &format!("{}: invalid mode: \"{}\"", name, mode));
            return Ok(());
        }
    };
    match sys::open_file(&vproc_path(vp, &path), &mode) {
        Ok(fd) => {
            let s = Value::Stream(new_stream(m, fd, smode));
            if let Some(r) = track {
                let tail = std::mem::replace(&mut *r.value.borrow_mut(), Value::Null);
                *r.value.borrow_mut() = Value::LList(Rc::new(LList { head: s.clone(), tail }));
                m.ref_changed = true;
            }
            vp.prim_return(vec![s]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), path)),
    }
    Ok(())
}

pub fn chdir(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_chdir__";
    let (on_error, path) = match path_args(vp, name, "path")? {
        Some(x) => x,
        None => return Ok(()),
    };
    let target = vproc_path(vp, &path);
    let r = sys::is_dir(&target).and_then(|dir| {
        if !dir {
            Err(io_error(20)) // ENOTDIR
        } else if !sys::can_search(&target) {
            Err(io_error(13)) // EACCES
        } else {
            sys::canonicalize(&target)
        }
    });
    match r {
        Ok(real) => {
            vp.cwd = real;
            vp.prim_return(vec![]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), path)),
    }
    Ok(())
}

pub fn getcwd(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let cwd = string_value(&vp.cwd);
    vp.prim_return(vec![cwd]);
    Ok(())
}

pub fn realpath(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_realpath__";
    let (on_error, path) = match path_args(vp, name, "path")? {
        Some(x) => x,
        None => return Ok(()),
    };
    let target = vproc_path(vp, &path);
    match sys::is_dir(&target).and_then(|_| sys::canonicalize(&target)) {
        Ok(real) => vp.prim_return(vec![string_value(&real)]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), path)),
    }
    Ok(())
}

pub fn listdir(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_listdir__";
    let (on_error, path) = match path_args(vp, name, "path")? {
        Some(x) => x,
        None => return Ok(()),
    };
    let r = sys::read_dir(&vproc_path(vp, &path));
    match r {
        Ok(mut names) => {
            names.sort();
            let mut list = Value::Null;
            for n in names.iter().rev() {
                list = Value::LList(Rc::new(LList { head: string_value(n), tail: list }));
            }
            vp.prim_return(vec![list]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), path)),
    }
    Ok(())
}

// ── The interpreter's process ───────────────────────────────────────────────

pub fn get_env_dict(m: &mut Machine, vp: &mut VProc) -> PResult {
    m.serials.dicts += 1;
    let d = DictObj { serial: m.serials.dicts, entries: RefCell::new(Vec::new()), index: RefCell::new(DictIndex::default()), shape: std::cell::Cell::new(SHAPE_EMPTY) };
    for (k, v) in sys::env_vars() {
        let k = string_value(&k);
        let v = string_value(&v);
        d.append(k, v).map_err(VmError::Error)?;
    }
    vp.prim_return(vec![Value::Dict(crate::gc::new_dict(d))]);
    Ok(())
}

pub fn getpid(_m: &mut Machine, vp: &mut VProc) -> PResult {
    vp.prim_return(vec![num(sys::getpid() as f64)]);
    Ok(())
}

pub fn sysconf(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_sysconf__";
    let (on_error, var) = match path_args(vp, name, "name")? {
        Some(x) => x,
        None => return Ok(()),
    };
    let n = match sys::sysconf_name(&var) {
        Some(n) => n,
        None => {
            vp.prim_error(on_error, &format!("{}: unknown name: {}", name, var));
            return Ok(());
        }
    };
    match sys::sysconf_value(n) {
        Ok(-1) => vp.prim_error(on_error, &format!("{}: no value: {}", name, var)),
        Ok(v) => vp.prim_return(vec![num(v as f64)]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), var)),
    }
    Ok(())
}
