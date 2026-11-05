//! OS primitives: printing and exiting.

use super::{take_args, PResult, VmError};
use crate::sched::Machine;
use crate::trace::value_to_display;
use crate::vproc::VProc;
use std::io::Write;

pub fn exit_normal(_m: &mut Machine, _vp: &mut VProc) -> PResult {
    Err(VmError::Exit(0))
}

pub fn exit_error(_m: &mut Machine, _vp: &mut VProc) -> PResult {
    Err(VmError::Exit(1))
}

/// Write text to fd 1 or 2 at once, so that the order of standard output and
/// standard error is kept when both go to the same file.
pub fn write_std(fd: i32, text: &str) {
    let r = if fd == 2 {
        let mut e = std::io::stderr().lock();
        e.write_all(text.as_bytes()).and_then(|_| e.flush())
    } else {
        let mut o = std::io::stdout().lock();
        o.write_all(text.as_bytes()).and_then(|_| o.flush())
    };
    // Like a process killed by SIGPIPE: stop when the reader has gone.
    if let Err(e) = r {
        if e.kind() == std::io::ErrorKind::BrokenPipe {
            std::process::exit(141);
        }
    }
}

pub fn print(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__OS_print__", 1)?;
    let s = value_to_display(&a[0]).map_err(VmError::Error)?;
    write_std(1, &s);
    vp.prim_return(vec![]);
    Ok(())
}

pub fn print_error(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__OS_print_error__", 1)?;
    let s = value_to_display(&a[0]).map_err(VmError::Error)?;
    write_std(2, &s);
    vp.prim_return(vec![]);
    Ok(())
}

/// __OS_exit__ onError Number ,(): exit with the code Number, an integer in 0..255.
pub fn exit(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__OS_exit__";
    let mut a = super::take_n(vp, name, 2)?;
    let code = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let x = super::number::check_number(name, &code)?;
    if !(x.is_finite() && x == x.trunc() && (0.0..=255.0).contains(&x)) {
        let msg = format!("{}: not an exit code: {}", name, crate::value::number_to_str(x));
        vp.prim_error(on_error, &msg);
        return Ok(());
    }
    Err(VmError::Exit(x as i32))
}
