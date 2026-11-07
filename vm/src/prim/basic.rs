//! __is_null__ and __type_name__.

use super::{take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

pub fn is_null(m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__is_null__", 1)?;
    let b = is_null_core(m, &a[0]);
    vp.prim_return(vec![b]);
    Ok(())
}

/// The result of __is_null__ (also for vm/src/transpiled.rs).
pub fn is_null_core(m: &Machine, v: &Value) -> Value {
    if v.is_null() {
        m.true_closure()
    } else {
        m.false_closure()
    }
}

pub fn type_name(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__type_name__", 1)?;
    let t = type_name_core(&a[0])?;
    vp.prim_return(vec![t]);
    Ok(())
}

/// The result of __type_name__.
pub fn type_name_core(v: &Value) -> Result<Value, VmError> {
    Ok(string_value(crate::value::type_name(v).map_err(VmError::Error)?))
}
