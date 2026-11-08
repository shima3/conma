//! OS process and signal primitives.

use super::{err, take_n, PResult, VmError};
use super::stream::check_stream;
use crate::sched::{Machine, OsTarget};
use crate::sys;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;
#[cfg(unix)]
use std::cell::{Cell, RefCell};
#[cfg(unix)]
use std::os::fd::{FromRawFd, OwnedFd};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
#[cfg(unix)]
use std::process::{Command, Stdio};
use std::rc::Rc;

pub fn spawn(m: &mut Machine, vp: &mut VProc) -> PResult {
    do_spawn(m, vp, "__OS_spawn__", false)
}

/// Like __OS_spawn__, but the process starts in a new session (setsid).
pub fn spawn_detached(m: &mut Machine, vp: &mut VProc) -> PResult {
    do_spawn(m, vp, "__OS_spawn_detached__", true)
}

/// __OS_spawn__ onError commandMSeq inputStream outputStream errorStream envDict ,(OSProc)
fn do_spawn(m: &mut Machine, vp: &mut VProc, name: &str, detached: bool) -> PResult {
    let mut a = take_n(vp, name, 6)?;
    let env_dict = a.pop().unwrap();
    let streams: Vec<Value> = a.split_off(2);
    let command = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let items = match &command {
        Value::MSeq(s) => s.items.borrow().clone(),
        other => {
            vp.prim_error(on_error, &format!("{}: expected MSeq command, got {}", name, value_type_name(other)));
            return Ok(());
        }
    };
    if items.is_empty() {
        vp.prim_error(on_error, &format!("{}: empty command", name));
        return Ok(());
    }
    let mut args = Vec::new();
    for item in &items {
        if value_type_name(item) != "String" {
            vp.prim_error(on_error, &format!("{}: expected String in command, got {}", name, value_type_name(item)));
            return Ok(());
        }
        args.push(value_to_display(item).map_err(VmError::Error)?);
    }
    let mut fds = Vec::new();
    for (s, mode) in streams.iter().zip(['r', 'w', 'w']) {
        if s.is_null() {
            fds.push(None); // inherit the interpreter's stream
        } else {
            match check_stream(vp, &on_error, name, s, Some(mode)) {
                Some(st) => fds.push(Some(st.fd)),
                None => return Ok(()),
            }
        }
    }
    let entries = match &env_dict {
        Value::Dict(d) => d.entries.borrow().clone(),
        other => {
            vp.prim_error(on_error, &format!("{}: expected Dict envDict, got {}", name, value_type_name(other)));
            return Ok(());
        }
    };
    let mut env = Vec::new();
    for (k, v) in &entries {
        if value_type_name(k) != "String" || value_type_name(v) != "String" {
            vp.prim_error(on_error, &format!("{}: envDict keys and values must be Strings", name));
            return Ok(());
        }
        env.push((value_to_display(k).map_err(VmError::Error)?, value_to_display(v).map_err(VmError::Error)?));
    }
    start_process(m, vp, name, detached, on_error, args, fds, env)
}

/// The VM for wasm32: the host starts the process (sys::spawn), if it can.
#[cfg(not(unix))]
#[allow(clippy::too_many_arguments)]
fn start_process(
    m: &mut Machine,
    vp: &mut VProc,
    name: &str,
    detached: bool,
    on_error: Value,
    args: Vec<String>,
    fds: Vec<Option<i32>>,
    env: Vec<(String, String)>,
) -> PResult {
    match sys::spawn(&args, &fds, &env, &vp.cwd, detached) {
        Ok(host) => {
            m.serials.osprocs += 1;
            let p = OSProcObj { serial: m.serials.osprocs, host, code: std::cell::Cell::new(None) };
            vp.prim_return(vec![Value::OSProc(Rc::new(p))]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), args[0])),
    }
    Ok(())
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn start_process(
    m: &mut Machine,
    vp: &mut VProc,
    name: &str,
    detached: bool,
    on_error: Value,
    args: Vec<String>,
    fds: Vec<Option<i32>>,
    env: Vec<(String, String)>,
) -> PResult {
    let mut cmd = Command::new(&args[0]);
    cmd.args(&args[1..]).env_clear().envs(env).current_dir(&vp.cwd);
    let mut stdio = Vec::new();
    for fd in &fds {
        stdio.push(match fd {
            None => Stdio::inherit(),
            Some(fd) => match sys::dup(*fd) {
                Ok(d) => Stdio::from(unsafe { OwnedFd::from_raw_fd(d) }),
                Err(e) => {
                    vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), args[0]));
                    return Ok(());
                }
            },
        });
    }
    let mut it = stdio.into_iter();
    cmd.stdin(it.next().unwrap()).stdout(it.next().unwrap()).stderr(it.next().unwrap());
    if detached {
        unsafe {
            cmd.pre_exec(|| {
                sys::setsid();
                Ok(())
            });
        }
    }
    match cmd.spawn() {
        Ok(child) => {
            m.serials.osprocs += 1;
            let p = OSProcObj { serial: m.serials.osprocs, child: RefCell::new(child), code: Cell::new(None) };
            vp.prim_return(vec![Value::OSProc(Rc::new(p))]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}: {}", name, sys::strerror(&e), args[0])),
    }
    Ok(())
}

/// onError and the OSProc argument; onError and None if it is not an OSProc.
fn proc_args(vp: &mut VProc, name: &str, n: usize) -> Result<Option<(Value, Rc<OSProcObj>, Vec<Value>)>, VmError> {
    let mut a = take_n(vp, name, n)?;
    let rest = a.split_off(2);
    let p = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    match p {
        Value::OSProc(p) => Ok(Some((on_error, p, rest))),
        other => {
            vp.prim_error(on_error, &format!("{}: expected OSProc, got {}", name, value_type_name(&other)));
            Ok(None)
        }
    }
}

fn code_value(c: Option<i32>) -> Value {
    match c {
        Some(c) => num(c as f64),
        None => Value::Null,
    }
}

pub fn poll(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_poll__";
    let (on_error, p, _) = match proc_args(vp, name, 2)? {
        Some(x) => x,
        None => return Ok(()),
    };
    match p.poll() {
        Ok(c) => vp.prim_return(vec![code_value(c)]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::errno_str(&e))),
    }
    Ok(())
}

/// Like __OS_poll__, but a running process makes the VProc wait until it exits.
pub fn wait(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_wait__";
    let (_, p, _) = match proc_args(vp, name, 2)? {
        Some(x) => x,
        None => return Ok(()),
    };
    // Install the SIGCHLD wakeup before polling, so that an exit after the
    // poll still wakes the wait for fds.
    m.install_sigchld();
    match p.poll().unwrap_or(None) {
        Some(c) => vp.prim_return(vec![num(c as f64)]),
        None => {
            vp.prim_return(vec![]); // the exit code is added when it wakes up
            let cur = m.current.clone().expect("no current VProc");
            m.wait_os(&cur, OsTarget::Proc(p));
        }
    }
    Ok(())
}

/// __OS_kill__ onError OSProc signal ,()
pub fn kill(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_kill__";
    let (on_error, p, rest) = match proc_args(vp, name, 3)? {
        Some(x) => x,
        None => return Ok(()),
    };
    let sig = match rest[0] {
        Value::Number(x) if x.is_finite() && x == x.trunc() && (1.0..=64.0).contains(&x) => x as i32,
        _ => {
            vp.prim_error(on_error, &format!("{}: expected a signal number from 1 to 64", name));
            return Ok(());
        }
    };
    // Nothing happens if the process has already been waited for (Popen.send_signal).
    let r = match p.poll() {
        Ok(Some(_)) => Ok(()),
        #[cfg(unix)]
        _ => sys::send_signal(p.child.borrow().id() as i32, sig),
        #[cfg(not(unix))]
        _ => sys::proc_kill(p.host, sig),
    };
    match r {
        Ok(()) => vp.prim_return(vec![]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::errno_str(&e))),
    }
    Ok(())
}

/// __OS_trap_interrupt__ Ref ,()
pub fn trap_interrupt(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_trap_interrupt__";
    if vp.olist.is_empty() {
        return err(format!("{}: missing argument", name));
    }
    let r = match vp.olist.remove(0) {
        Value::Ref(r) => r,
        other => return err(format!("{}: expected Ref, got {}", name, value_type_name(&other))),
    };
    m.trap_interrupt(r);
    vp.prim_return(vec![]);
    Ok(())
}
