//! The feature "array" of extended ConMa (docs/dev/ARRAY-PLAN-ja.md): arrays read and written by
//! index. The replacement definitions of core ConMa (include/core/array.se) keep an array in a
//! Dict, and give the same results and the same messages to onError; only __type_name__ and the
//! errors of values that are not arrays given to __Array_length__ and __Array_push__ differ.

use super::{err, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::rc::Rc;

/// The index that `i` is in an array of `len` values, or the end of the message of onError
/// (after the name of the primitive).
pub fn index(i: &Value, len: usize) -> Result<usize, &'static str> {
    let Value::Number(x) = i else { return Err(": index is not a Number") };
    let x = *x;
    if x.floor() != x || !x.is_finite() {
        return Err(": index is not an integer");
    }
    if x < 0.0 || x >= len as f64 {
        return Err(": index out of range");
    }
    Ok(x as usize)
}

/// The array an operand is, or the message of onError.
fn array_of(name: &str, v: &Value) -> Result<Rc<ArrayObj>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        _ => Err(format!("{}: not an Array", name)),
    }
}

/// The length of an array (the error stops the VProc).
pub fn length_core(a: &Value) -> Result<usize, VmError> {
    match a {
        Value::Array(arr) => Ok(arr.items.borrow().len()),
        v => err(format!("__Array_length__: expected Array, got {}", value_type_name(v))),
    }
}

/// The value at index i, or the message of onError.
pub fn get_core(a: &Value, i: &Value) -> Result<Value, String> {
    let arr = array_of("__Array_get__", a)?;
    let items = arr.items.borrow();
    match index(i, items.len()) {
        Ok(k) => Ok(items[k].clone()),
        Err(msg) => Err(format!("__Array_get__{}", msg)),
    }
}

/// Set the value at index i, or the message of onError.
pub fn set_core(a: &Value, i: &Value, value: Value) -> Result<(), String> {
    let arr = array_of("__Array_set__", a)?;
    let len = arr.items.borrow().len();
    match index(i, len) {
        Ok(k) => {
            // the old value is dropped after the borrow ends (dropping it may drop this array)
            let old = std::mem::replace(&mut arr.items.borrow_mut()[k], value);
            drop(old);
            Ok(())
        }
        Err(msg) => Err(format!("__Array_set__{}", msg)),
    }
}

/// Append a value (the error stops the VProc).
pub fn push_core(a: &Value, value: Value) -> Result<(), VmError> {
    match a {
        Value::Array(arr) => {
            arr.items.borrow_mut().push(value);
            Ok(())
        }
        v => err(format!("__Array_push__: expected Array, got {}", value_type_name(v))),
    }
}

pub fn new(m: &mut Machine, vp: &mut VProc) -> PResult {
    m.serials.arrays += 1;
    let a = crate::gc::new_array(ArrayObj { serial: m.serials.arrays, items: RefCell::new(Vec::new()) });
    vp.prim_return1(Value::Array(a));
    Ok(())
}

pub fn length(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Array_length__", 1)?;
    let n = length_core(&a[0])?;
    vp.prim_return1(Value::Number(n as f64));
    Ok(())
}

pub fn get(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Array_get__", 3)?;
    match get_core(&a[1], &a[2]) {
        Ok(v) => {
            drop(a);
            vp.prim_return1(v)
        }
        Err(msg) => vp.prim_error(a[0].clone(), &msg),
    }
    Ok(())
}

pub fn set(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Array_set__", 4)?;
    let value = a.pop().unwrap();
    match set_core(&a[1], &a[2], value) {
        Ok(()) => {
            drop(a);
            vp.prim_return0()
        }
        Err(msg) => vp.prim_error(a[0].clone(), &msg),
    }
    Ok(())
}

pub fn push(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Array_push__", 2)?;
    let value = a.pop().unwrap();
    push_core(&a[0], value)?;
    drop(a);
    vp.prim_return0();
    Ok(())
}

pub fn pop(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Array_pop__", 2)?;
    let arr = match array_of("__Array_pop__", &a[1]) {
        Ok(arr) => arr,
        Err(msg) => {
            vp.prim_error(a[0].clone(), &msg);
            return Ok(());
        }
    };
    let last = arr.items.borrow_mut().pop();
    match last {
        Some(v) => {
            drop(a);
            vp.prim_return1(v)
        }
        None => vp.prim_error(a[0].clone(), "__Array_pop__: empty Array"),
    }
    Ok(())
}
