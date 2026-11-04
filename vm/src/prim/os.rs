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
    if fd == 2 {
        let mut e = std::io::stderr().lock();
        let _ = e.write_all(text.as_bytes());
        let _ = e.flush();
    } else {
        let mut o = std::io::stdout().lock();
        let _ = o.write_all(text.as_bytes());
        let _ = o.flush();
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
