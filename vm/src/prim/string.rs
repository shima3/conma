//! String primitives and __Sexp_read__. Indices count code points.

use super::number::{check_number, is_number_syntax, parse_number};
use super::{err, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// Take count Strings and return their text; fail if one is not a String.
fn string_args(vp: &mut VProc, name: &str, count: usize) -> Result<Vec<String>, VmError> {
    if vp.olist.len() < count {
        return err(format!("{}: missing argument(s)", name));
    }
    let args: Vec<Value> = vp.olist.drain(..count).collect();
    for a in &args {
        if value_type_name(a) != "String" {
            return err(format!("{}: expected String, got {}", name, value_type_name(a)));
        }
    }
    args.iter().map(|a| value_to_display(a).map_err(VmError::Error)).collect()
}

fn need(vp: &VProc, name: &str, n: usize) -> PResult {
    if vp.olist.len() < n {
        return err(format!("{}: missing argument(s)", name));
    }
    Ok(())
}

/// An integer index, or None if x is not a finite integer.
fn int_index(x: f64) -> Option<i128> {
    if x.is_finite() && x == x.trunc() {
        Some(x as i128)
    } else {
        None
    }
}

/// Take count Strings (checked in order) and return them as they are, so that
/// their text can be read without copying it (text).
fn string_values(vp: &mut VProc, name: &str, count: usize) -> Result<Vec<Value>, VmError> {
    if vp.olist.len() < count {
        return err(format!("{}: missing argument(s)", name));
    }
    let args: Vec<Value> = vp.olist.drain(..count).collect();
    for a in &args {
        if value_type_name(a) != "String" {
            return err(format!("{}: expected String, got {}", name, value_type_name(a)));
        }
    }
    Ok(args)
}

/// The text of a String, without copying a String made at run time.
fn text(v: &Value) -> Result<std::borrow::Cow<'_, str>, VmError> {
    match v {
        Value::Str(s) => Ok(std::borrow::Cow::Borrowed(&**s)),
        _ => Ok(std::borrow::Cow::Owned(value_to_display(v).map_err(VmError::Error)?)),
    }
}

/// The number of code points of s (the length of a String).
fn char_count(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

/// The byte offset of the code point at char index i, or None if s has fewer
/// than i code points (i == the length gives s.len()). When the first i bytes
/// are ASCII, it is i, found without walking the string.
fn char_to_byte(s: &str, i: usize) -> Option<usize> {
    let b = s.as_bytes();
    if i <= b.len() && b[..i].is_ascii() {
        return Some(i);
    }
    let mut n = 0;
    for (bi, _) in s.char_indices() {
        if n == i {
            return Some(bi);
        }
        n += 1;
    }
    if n == i {
        Some(s.len())
    } else {
        None
    }
}

/// The number of code points of a part of a string (ASCII: its length).
fn chars_in(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.chars().count()
    }
}

pub fn length(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = string_values(vp, "__String_length__", 1)?;
    let t = text(&s[0])?;
    vp.prim_return(vec![num(char_count(&t) as f64)]);
    Ok(())
}

pub fn concat(_m: &mut Machine, vp: &mut VProc) -> PResult {
    need(vp, "__String_concat__", 2)?;
    let a: Vec<Value> = vp.olist.drain(..2).collect();
    let v = concat_core(&a[0], &a[1])?;
    vp.prim_return(vec![v]);
    Ok(())
}

pub fn equal(m: &mut Machine, vp: &mut VProc) -> PResult {
    need(vp, "__String_equal__", 2)?;
    let a: Vec<Value> = vp.olist.drain(..2).collect();
    let b = equal_core(m, &a[0], &a[1])?;
    vp.prim_return(vec![b]);
    Ok(())
}

/// The text of each of two operands of name, which must be Strings (checked
/// in order, as string_args does), without copying a String made at run time.
fn two_texts<'a>(name: &str, a: &'a Value, b: &'a Value) -> Result<[std::borrow::Cow<'a, str>; 2], VmError> {
    for v in [a, b] {
        if value_type_name(v) != "String" {
            return err(format!("{}: expected String, got {}", name, value_type_name(v)));
        }
    }
    let text = |v: &'a Value| -> Result<std::borrow::Cow<'a, str>, VmError> {
        match v {
            Value::Str(s) => Ok(std::borrow::Cow::Borrowed(&**s)),
            _ => Ok(std::borrow::Cow::Owned(value_to_display(v).map_err(VmError::Error)?)),
        }
    };
    Ok([text(a)?, text(b)?])
}

/// The result of __String_concat__ (also for vm/src/transpiled.rs).
pub fn concat_core(a: &Value, b: &Value) -> Result<Value, VmError> {
    let [x, y] = two_texts("__String_concat__", a, b)?;
    let mut s = String::with_capacity(x.len() + y.len());
    s.push_str(&x);
    s.push_str(&y);
    Ok(string_value(&s))
}

/// The result of __String_equal__ (also for vm/src/transpiled.rs).
pub fn equal_core(m: &Machine, a: &Value, b: &Value) -> Result<Value, VmError> {
    let [x, y] = two_texts("__String_equal__", a, b)?;
    Ok(if x == y { m.true_closure() } else { m.false_closure() })
}

pub fn substring(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_substring__";
    need(vp, name, 4)?;
    let on_error = vp.olist.remove(0);
    let sv = string_values(vp, name, 1)?;
    let s = text(&sv[0])?;
    let mut a = take_args(vp, name, 2)?;
    let end = a.pop().unwrap();
    let start = a.pop().unwrap();
    let x0 = check_number(name, &start)?;
    let x1 = check_number(name, &end)?;
    let mut idx = [0i128; 2];
    for (k, x) in [x0, x1].iter().enumerate() {
        match int_index(*x) {
            Some(i) => idx[k] = i,
            None => {
                vp.prim_error(on_error, &format!("{}: not an integer index: {}", name, number_to_str(*x)));
                return Ok(());
            }
        }
    }
    let (i, j) = (idx[0], idx[1]);
    let bj = if 0 <= i && i <= j && j <= s.len() as i128 { char_to_byte(&s, j as usize) } else { None };
    let Some(bj) = bj else {
        let len = char_count(&s) as i128;
        vp.prim_error(on_error, &format!("{}: index out of range: {} {} (length {})", name, i, j, len));
        return Ok(());
    };
    let bi = char_to_byte(&s, i as usize).expect("i <= j");
    vp.prim_return(vec![string_value(&s[bi..bj])]);
    Ok(())
}

pub fn find(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_find__";
    need(vp, name, 4)?;
    let on_error = vp.olist.remove(0);
    let sp = string_values(vp, name, 2)?;
    let s = text(&sp[0])?;
    let pattern = text(&sp[1])?;
    let start = take_args(vp, name, 1)?.pop().unwrap();
    let x = check_number(name, &start)?;
    let i = match int_index(x) {
        Some(i) => i,
        None => {
            vp.prim_error(on_error, &format!("{}: not an integer index: {}", name, number_to_str(x)));
            return Ok(());
        }
    };
    let b = if 0 <= i && i <= s.len() as i128 { char_to_byte(&s, i as usize) } else { None };
    let Some(b) = b else {
        let len = char_count(&s) as i128;
        vp.prim_error(on_error, &format!("{}: index out of range: {} (length {})", name, i, len));
        return Ok(());
    };
    let r = match s[b..].find(&*pattern) {
        None => Value::Null,
        Some(off) => num((i as usize + chars_in(&s[b..b + off])) as f64),
    };
    vp.prim_return(vec![r]);
    Ok(())
}

/// __String_find_any__ onError String chars start ,(index): the position of the first
/// character at or after start that is one of the characters of chars, or null.
pub fn find_any(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_find_any__";
    need(vp, name, 4)?;
    let on_error = vp.olist.remove(0);
    let sp = string_values(vp, name, 2)?;
    let s = text(&sp[0])?;
    let chars = text(&sp[1])?;
    let start = take_args(vp, name, 1)?.pop().unwrap();
    let x = check_number(name, &start)?;
    let i = match int_index(x) {
        Some(i) => i,
        None => {
            vp.prim_error(on_error, &format!("{}: not an integer index: {}", name, number_to_str(x)));
            return Ok(());
        }
    };
    let b = if 0 <= i && i <= s.len() as i128 { char_to_byte(&s, i as usize) } else { None };
    let Some(b) = b else {
        let len = char_count(&s) as i128;
        vp.prim_error(on_error, &format!("{}: index out of range: {} (length {})", name, i, len));
        return Ok(());
    };
    let r = match s[b..].chars().position(|c| chars.contains(c)) {
        None => Value::Null,
        Some(off) => num((i as usize + off) as f64),
    };
    vp.prim_return(vec![r]);
    Ok(())
}

/// __String_strip_suffix__ String suffix ,(String): String without suffix at its end,
/// or String itself if it does not end with suffix.
pub fn strip_suffix(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_strip_suffix__";
    need(vp, name, 2)?;
    let sp = string_values(vp, name, 2)?;
    let s = text(&sp[0])?;
    let suffix = text(&sp[1])?;
    let r = match s.strip_suffix(&*suffix) {
        Some(t) if !suffix.is_empty() => string_value(t),
        _ => sp[0].clone(),
    };
    vp.prim_return(vec![r]);
    Ok(())
}

pub fn to_code(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_toCode__";
    need(vp, name, 2)?;
    let on_error = vp.olist.remove(0);
    let sv = string_values(vp, name, 1)?;
    let s = text(&sv[0])?;
    let mut it = s.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => vp.prim_return(vec![num(c as u32 as f64)]),
        _ => {
            let msg = format!("{}: expected one character, got length {}", name, s.chars().count());
            vp.prim_error(on_error, &msg)
        }
    }
    Ok(())
}

pub fn from_code(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__String_fromCode__";
    let mut a = take_args(vp, name, 2)?;
    let code = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let x = check_number(name, &code)?;
    let c = if x.is_finite() && x == x.trunc() && (0.0..=1114111.0).contains(&x) {
        char::from_u32(x as u32)
    } else {
        None
    };
    match c {
        Some(c) => vp.prim_return(vec![string_value(&c.to_string())]),
        None => vp.prim_error(on_error, &format!("{}: not a code point: {}", name, number_to_str(x))),
    }
    Ok(())
}

// ── __Sexp_read__ ───────────────────────────────────────────────────────────

struct SexpError(&'static str, usize);

fn is_delim(c: Option<char>) -> bool {
    matches!(c, None | Some(' ') | Some('\t') | Some('\n') | Some('\r') | Some('(') | Some(')') | Some('"'))
}

fn skip(t: &[char], mut i: usize) -> usize {
    while i < t.len() && matches!(t[i], ' ' | '\t' | '\n' | '\r') {
        i += 1;
    }
    i
}

fn atom(t: &[char], mut i: usize) -> Result<(Value, usize), SexpError> {
    let ch = t.get(i).copied();
    if ch == Some('"') {
        let mut out = String::new();
        i += 1;
        loop {
            match t.get(i).copied() {
                None => return Err(SexpError("unterminated string", i)),
                Some('"') => return Ok((string_value(&out), i + 1)),
                Some('\\') => {
                    let e = match t.get(i + 1).copied() {
                        None => return Err(SexpError("unterminated string", i + 1)),
                        Some(e) => e,
                    };
                    out.push(match e {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        e => e,
                    });
                    i += 2;
                }
                Some(c) => {
                    out.push(c);
                    i += 1;
                }
            }
        }
    }
    if is_delim(ch) {
        return Err(SexpError("expected a value", i));
    }
    let start = i;
    while !is_delim(t.get(i).copied()) {
        i += 1;
    }
    let symbol: String = t[start..i].iter().collect();
    if t[start].is_ascii_digit() {
        if !is_number_syntax(&symbol) || parse_number(&symbol).is_infinite() {
            return Err(SexpError("invalid number", start));
        }
        return Ok((num(parse_number(&symbol)), i));
    }
    Ok((string_value(&symbol), i))
}

fn sexp_value(t: &[char], mut i: usize) -> Result<(Value, usize), SexpError> {
    let mut stack: Vec<Vec<Value>> = Vec::new();
    loop {
        i = skip(t, i);
        let ch = t.get(i).copied();
        let value;
        if ch == Some('(') {
            stack.push(Vec::new());
            i += 1;
            continue;
        }
        if !stack.is_empty() && ch == Some(')') {
            let items = stack.pop().unwrap();
            i += 1;
            let mut v = Value::Null;
            for item in items.into_iter().rev() {
                v = Value::LList(Rc::new(LList { head: item, tail: v }));
            }
            value = v;
        } else if !stack.is_empty() && ch.is_none() {
            return Err(SexpError("unterminated list", i));
        } else {
            let (v, j) = atom(t, i)?;
            value = v;
            i = j;
        }
        match stack.last_mut() {
            None => return Ok((value, i)),
            Some(top) => top.push(value),
        }
    }
}

pub fn sexp_read(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Sexp_read__";
    need(vp, name, 2)?;
    let on_error = vp.olist.remove(0);
    let text: Vec<char> = string_args(vp, name, 1)?.pop().unwrap().chars().collect();
    let r = sexp_value(&text, 0).and_then(|(v, i)| {
        let i = skip(&text, i);
        if i != text.len() {
            Err(SexpError("unexpected characters after an S-expression", i))
        } else {
            Ok(v)
        }
    });
    match r {
        Ok(v) => vp.prim_return(vec![v]),
        Err(SexpError(msg, i)) => vp.prim_error(on_error, &format!("{} at column {}", msg, i + 1)),
    }
    Ok(())
}
