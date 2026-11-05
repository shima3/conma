//! LList primitives.

use super::{take_args, PResult};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

pub fn cons(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__LList_cons__", 2)?;
    let tail = a.pop().unwrap();
    let head = a.pop().unwrap();
    vp.prim_return(vec![Value::LList(Rc::new(LList { head, tail }))]);
    Ok(())
}

pub fn uncons(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__LList_uncons__", 2)?;
    let lst = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    match &lst {
        Value::LList(cell) => {
            let r = vec![cell.head.clone(), cell.tail.clone()];
            vp.prim_return(r);
        }
        _ => {
            let msg = format!("__LList_uncons__: expected LList, got {}", value_type_name(&lst));
            vp.prim_error(on_error, &msg);
        }
    }
    Ok(())
}
