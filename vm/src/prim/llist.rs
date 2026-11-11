//! LList primitives.

use super::{check_args, take_args, take_at, PResult};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

pub fn cons(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__LList_cons__", 2)?;
    let tail = take_at(vp, 1);
    let head = take_at(vp, 0);
    vp.prim_return1(Value::LList(Rc::new(LList { head, tail })));
    Ok(())
}

/// The head and tail of an LList, or the message for onError (also for
/// vm/src/transpiled.rs).
pub fn uncons_core(lst: &Value) -> Result<(Value, Value), String> {
    match lst {
        Value::LList(cell) => Ok((cell.head.clone(), cell.tail.clone())),
        _ => Err(format!("__LList_uncons__: expected LList, got {}", value_type_name(lst))),
    }
}

pub fn uncons(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__LList_uncons__", 2)?;
    let lst = take_at(vp, 1);
    let on_error = take_at(vp, 0);
    vp.olist.drain(..2);
    match &lst {
        Value::LList(cell) => {
            vp.prim_return2(cell.head.clone(), cell.tail.clone());
        }
        _ => {
            let msg = format!("__LList_uncons__: expected LList, got {}", value_type_name(&lst));
            vp.prim_error(on_error, &msg);
        }
    }
    Ok(())
}

/// The cells of a list that ends with null, or an error that names the primitive.
fn cells(name: &str, list: &Value) -> Result<Vec<Value>, super::VmError> {
    let mut items = Vec::new();
    let mut cur = list.clone();
    loop {
        match cur {
            Value::Null => return Ok(items),
            Value::LList(cell) => {
                items.push(cell.head.clone());
                cur = cell.tail.clone();
            }
            other => return super::err(format!("{}: expected LList, got {}", name, value_type_name(&other))),
        }
    }
}

/// __LList_append__ a b ,(list): the elements of a followed by b (b is not copied).
pub fn append(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__LList_append__", 2)?;
    let b = a.pop().unwrap();
    let front = cells("__LList_append__", &a.pop().unwrap())?;
    let mut list = b;
    for head in front.into_iter().rev() {
        list = Value::LList(Rc::new(LList { head, tail: list }));
    }
    vp.prim_return(vec![list]);
    Ok(())
}

/// __LList_length__ list ,(Number)
pub fn length(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__LList_length__", 1)?;
    let n = cells("__LList_length__", &a.pop().unwrap())?.len();
    vp.prim_return(vec![num(n as f64)]);
    Ok(())
}
