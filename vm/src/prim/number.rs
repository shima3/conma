//! Number primitives.

use super::{err, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;

pub fn check_number(name: &str, v: &Value) -> Result<f64, VmError> {
    match v {
        Value::Number(x, _) => Ok(*x),
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

pub fn to_string(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Number_toString__", 1)?;
    let x = check_number("__Number_toString__", &a[0])?;
    vp.prim_return(vec![string_value(&number_to_str(x))]);
    Ok(())
}

fn predicate(m: &mut Machine, vp: &mut VProc, name: &str, f: fn(f64) -> bool) -> PResult {
    let a = take_args(vp, name, 1)?;
    let x = check_number(name, &a[0])?;
    let b = if f(x) { m.true_closure() } else { m.false_closure() };
    vp.prim_return(vec![b]);
    Ok(())
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
    let a = take_args(vp, name, 2)?;
    let x = check_number(name, &a[0])?;
    let y = check_number(name, &a[1])?;
    vp.prim_return(vec![num(f(x, y))]);
    Ok(())
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
    let mut a = take_args(vp, "__Number_divide__", 3)?;
    let y = a.pop().unwrap();
    let x = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let x = check_number("__Number_divide__", &x)?;
    let y = check_number("__Number_divide__", &y)?;
    if y == 0.0 {
        vp.prim_error(on_error, "__Number_divide__: division by zero");
        return Ok(());
    }
    vp.prim_return(vec![num(x / y)]);
    Ok(())
}

pub fn floor(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Number_floor__", 1)?;
    let x = check_number("__Number_floor__", &a[0])?;
    vp.prim_return(vec![num(if x.is_finite() { x.floor() } else { x })]);
    Ok(())
}
