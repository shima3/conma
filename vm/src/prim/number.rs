//! Number primitives.

use super::{check_args, err, in_place_failed, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;

pub fn check_number(name: &str, v: &Value) -> Result<f64, VmError> {
    match v {
        Value::Number(x) => Ok(*x),
        _ => err(format!("{}: expected Number, got {}", name, value_type_name(v))),
    }
}

/// The syntax `[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?`, the whole text.
pub fn is_number_syntax(s: &str) -> bool {
    let b = s.as_bytes();
    let mut i = 0;
    let digits = |i: &mut usize| {
        let st = *i;
        while *i < b.len() && b[*i].is_ascii_digit() {
            *i += 1;
        }
        *i - st
    };
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let int_digits = digits(&mut i);
    if int_digits > 0 {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            digits(&mut i);
        }
    } else {
        if i < b.len() && b[i] == b'.' {
            i += 1;
        } else {
            return false;
        }
        if digits(&mut i) == 0 {
            return false;
        }
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let save = i;
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        if digits(&mut i) == 0 {
            i = save;
        }
    }
    i == b.len()
}

/// The float of a text in the number syntax.
pub fn parse_number(s: &str) -> f64 {
    s.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn from_string(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Number_fromString__", 2)?;
    let val = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    if value_type_name(&val) != "String" {
        let msg = format!("__Number_fromString__: expected String, got {}", value_type_name(&val));
        vp.prim_error(on_error, &msg);
        return Ok(());
    }
    let s = value_to_display(&val).map_err(VmError::Error)?;
    if !is_number_syntax(&s) {
        vp.prim_error(on_error, &format!("__Number_fromString__: invalid number: \"{}\"", s));
        return Ok(());
    }
    let n = parse_number(&s);
    if n.is_infinite() {
        vp.prim_error(on_error, &format!("__Number_fromString__: out of range: \"{}\"", s));
        return Ok(());
    }
    vp.prim_return(vec![num(n)]);
    Ok(())
}

// The primitives below read their operands in place (check_args) and return
// through the OList (VProc::prim_return1), so that they make no Vec.

pub fn to_string(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__Number_toString__", 1)?;
    let x = match check_number("__Number_toString__", &vp.olist[0]) {
        Ok(x) => x,
        Err(e) => return in_place_failed(vp, 1, e),
    };
    vp.prim_return1(string_value(&number_to_str(x)));
    Ok(())
}

fn predicate(m: &mut Machine, vp: &mut VProc, name: &str, f: fn(f64) -> bool) -> PResult {
    check_args(vp, name, 1)?;
    match predicate_core(m, name, &vp.olist[0], f) {
        Ok(b) => vp.prim_return1(b),
        Err(e) => return in_place_failed(vp, 1, e),
    }
    Ok(())
}

/// The result of a predicate primitive (also for vm/src/transpiled.rs).
pub fn predicate_core(m: &Machine, name: &str, v: &Value, f: fn(f64) -> bool) -> Result<Value, VmError> {
    let x = check_number(name, v)?;
    Ok(if f(x) { m.true_closure() } else { m.false_closure() })
}

pub fn is_zero(m: &mut Machine, vp: &mut VProc) -> PResult {
    predicate(m, vp, "__Number_is_zero__", |x| x == 0.0)
}
pub fn is_positive(m: &mut Machine, vp: &mut VProc) -> PResult {
    predicate(m, vp, "__Number_is_positive__", |x| x > 0.0)
}
pub fn is_negative(m: &mut Machine, vp: &mut VProc) -> PResult {
    predicate(m, vp, "__Number_is_negative__", |x| x < 0.0)
}

fn binop(vp: &mut VProc, name: &str, f: fn(f64, f64) -> f64) -> PResult {
    check_args(vp, name, 2)?;
    match binop_core(name, &vp.olist[0], &vp.olist[1], f) {
        Ok(v) => vp.prim_return1(v),
        Err(e) => return in_place_failed(vp, 2, e),
    }
    Ok(())
}

/// The result of a binary Number primitive (also for vm/src/transpiled.rs).
pub fn binop_core(name: &str, a: &Value, b: &Value, f: fn(f64, f64) -> f64) -> Result<Value, VmError> {
    let x = check_number(name, a)?;
    let y = check_number(name, b)?;
    Ok(num(f(x, y)))
}

pub fn add(_m: &mut Machine, vp: &mut VProc) -> PResult {
    binop(vp, "__Number_add__", |a, b| a + b)
}
pub fn subtract(_m: &mut Machine, vp: &mut VProc) -> PResult {
    binop(vp, "__Number_subtract__", |a, b| a - b)
}
pub fn multiply(_m: &mut Machine, vp: &mut VProc) -> PResult {
    binop(vp, "__Number_multiply__", |a, b| a * b)
}

pub fn divide(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Number_divide__";
    check_args(vp, name, 3)?;
    let xy = check_number(name, &vp.olist[1]).and_then(|x| check_number(name, &vp.olist[2]).map(|y| (x, y)));
    let (x, y) = match xy {
        Ok(xy) => xy,
        Err(e) => return in_place_failed(vp, 3, e),
    };
    if y == 0.0 {
        let on_error = std::mem::replace(&mut vp.olist[0], Value::Null);
        vp.olist.drain(..3);
        vp.prim_error(on_error, "__Number_divide__: division by zero");
        return Ok(());
    }
    vp.prim_return1(num(x / y));
    Ok(())
}

/// __Number_sqrt__ onError Number ,(Number): the square root; a negative
/// Number (not -0) applies onError.
pub fn sqrt(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__Number_sqrt__", 2)?;
    let x = match check_number("__Number_sqrt__", &vp.olist[1]) {
        Ok(x) => x,
        Err(e) => return in_place_failed(vp, 2, e),
    };
    if x < 0.0 {
        let on_error = std::mem::replace(&mut vp.olist[0], Value::Null);
        vp.olist.drain(..2);
        vp.prim_error(on_error, "__Number_sqrt__: square root of a negative number");
        return Ok(());
    }
    vp.prim_return1(num(x.sqrt()));
    Ok(())
}

pub fn floor(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_args(vp, "__Number_floor__", 1)?;
    let x = match check_number("__Number_floor__", &vp.olist[0]) {
        Ok(x) => x,
        Err(e) => return in_place_failed(vp, 1, e),
    };
    vp.prim_return1(num(if x.is_finite() { x.floor() } else { x }));
    Ok(())
}
