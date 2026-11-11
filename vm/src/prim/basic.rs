//! __is_null__ and __type_name__.

use super::{check_args, in_place_failed, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

pub fn is_null(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__is_null__", 1)?;
    let b = is_null_core(m, &vp.olist[0]);
    vp.prim_return1(b);
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
    check_args(vp, "__type_name__", 1)?;
    match type_name_core(&vp.olist[0]) {
        Ok(t) => vp.prim_return1(t),
        Err(e) => return in_place_failed(vp, 1, e),
    }
    Ok(())
}

/// The result of __type_name__.
pub fn type_name_core(v: &Value) -> Result<Value, VmError> {
    Ok(string_value(crate::value::type_name(v).map_err(VmError::Error)?))
}
