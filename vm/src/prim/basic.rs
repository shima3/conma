//! __is_null__ and __type_name__.

use super::{take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

pub fn is_null(m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__is_null__", 1)?;
    let b = if a[0].is_null() { m.true_closure() } else { m.false_closure() };
    vp.prim_return(vec![b]);
    Ok(())
}

pub fn type_name(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__type_name__", 1)?;
    let t = crate::value::type_name(&a[0]).map_err(VmError::Error)?;
    vp.prim_return(vec![string_value(t)]);
    Ok(())
}
