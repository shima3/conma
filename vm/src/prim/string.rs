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

/// The text of a String, without copying it when it can be borrowed (crate::value::string_cow).
fn text(v: &Value) -> Result<std::borrow::Cow<'_, str>, VmError> {
    match crate::value::string_cow(v) {
        Some(r) => r.map_err(VmError::Error),
        None => Ok(std::borrow::Cow::Owned(value_to_display(v).map_err(VmError::Error)?)),
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

/// What a text is made of (StrInfo, docs/dev/STRING-INDEX-PLAN-ja.md), found by looking at all of it.
fn info_of(t: &str) -> StrInfo {
    if t.is_ascii() {
        StrInfo::Ascii
    } else {
        StrInfo::Chars(t.chars().count())
    }
}

/// What is known of the characters of a String value without looking at a String made at run time
/// (Unknown if it has not been looked at); a String of the program (a short literal) is looked at.
fn known(v: &Value, t: &str) -> StrInfo {
    match v {
        Value::Str(s) => s.info.get(),
        _ => info_of(t),
    }
}

/// What a String value is made of: what is known of a String made at run time, or found once and kept
/// in it (t is its text).
fn info(v: &Value, t: &str) -> StrInfo {
    match v {
        Value::Str(s) => match s.info.get() {
            StrInfo::Unknown => {
                let i = info_of(t);
                s.info.set(i);
                i
            }
            i => i,
        },
        _ => info_of(t),
    }
}

/// The number of characters of a text whose StrInfo is known.
fn count(t: &str, i: StrInfo) -> usize {
    match i {
        StrInfo::Chars(n) => n,
        _ => t.len(),
    }
}

/// char_to_byte for the text t of the String value v, whose StrInfo is known: in an ASCII text, a
/// position in characters is the same in bytes. In a String made at run time that is not all ASCII,
/// the position is found by walking from the last position found in it (StrObj::mark), forward or
/// back, or from the start when that is nearer, and kept as the mark: reading a long String by
/// positions that move a little at a time (a lexer) takes time proportional to its length, not to
/// its length squared (docs/dev/STRING-INDEX-PLAN-ja.md, section 7).
fn char_to_byte_at(v: &Value, t: &str, i: usize, info: StrInfo) -> Option<usize> {
    match info {
        StrInfo::Ascii => return if i <= t.len() { Some(i) } else { None },
        StrInfo::Chars(n) if i > n => return None,
        _ => {}
    }
    let Value::Str(s) = v else { return char_to_byte(t, i) };
    let t: &str = &s.text;
    let (mc, mb) = s.mark.get();
    let b = if i >= mc {
        walk_forward(t, mb, i - mc)
    } else if mc - i < i {
        t[..mb].char_indices().rev().nth(mc - i - 1).map(|(o, _)| o)
    } else {
        char_to_byte(t, i)
    };
    if let Some(b) = b {
        s.mark.set((i, b));
    }
    b
}

/// The byte offset of the code point k code points after the byte offset from of t (t.len() just
/// after the last), or None if t has fewer.
fn walk_forward(t: &str, from: usize, k: usize) -> Option<usize> {
    let rest = &t[from..];
    let mut n = 0;
    for (o, _) in rest.char_indices() {
        if n == k {
            return Some(from + o);
        }
        n += 1;
    }
    if n == k {
        Some(t.len())
    } else {
        None
    }
}

pub fn length(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = string_values(vp, "__String_length__", 1)?;
    let t = text(&s[0])?;
    let i = info(&s[0], &t);
    vp.prim_return(vec![num(count(&t, i) as f64)]);
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
    match equal_core(m, &vp.olist[0], &vp.olist[1]) {
        Ok(b) => vp.prim_return1(b),
        Err(e) => return super::in_place_failed(vp, 2, e),
    }
    Ok(())
}

/// The text of each of two operands of name, which must be Strings (checked
/// in order, as string_args does), without copying a String made at run time.
fn two_texts<'a>(name: &str, a: &'a Value, b: &'a Value) -> Result<[std::borrow::Cow<'a, str>; 2], VmError> {
    // Both are checked to be Strings before either text is made (the error names the first that is not).
    let (ta, tb) = (crate::value::string_cow(a), crate::value::string_cow(b));
    let (Some(ta), Some(tb)) = (ta, tb) else {
        let v = if crate::value::string_cow(a).is_none() { a } else { b };
        return err(format!("{}: expected String, got {}", name, value_type_name(v)));
    };
    Ok([ta.map_err(VmError::Error)?, tb.map_err(VmError::Error)?])
}

/// The result of __String_concat__ (also for vm/src/transpiled.rs).
pub fn concat_core(a: &Value, b: &Value) -> Result<Value, VmError> {
    let [x, y] = two_texts("__String_concat__", a, b)?;
    let mut s = String::with_capacity(x.len() + y.len());
    s.push_str(&x);
    s.push_str(&y);
    // What is known of both is known of the two put together
    let i = match (known(a, &x), known(b, &y)) {
        (StrInfo::Ascii, StrInfo::Ascii) => StrInfo::Ascii,
        (StrInfo::Unknown, _) | (_, StrInfo::Unknown) => StrInfo::Unknown,
        (p, q) => StrInfo::Chars(count(&x, p) + count(&y, q)),
    };
    Ok(string_value_info(&s, i))
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
    let si = info(&sv[0], &s);
    // The start first, so that the end is found by walking from it (and the mark is left at the end,
    // where reading by positions goes on)
    let found = if 0 <= i && i <= j && j <= s.len() as i128 {
        char_to_byte_at(&sv[0], &s, i as usize, si)
            .and_then(|bi| char_to_byte_at(&sv[0], &s, j as usize, si).map(|bj| (bi, bj)))
    } else {
        None
    };
    let Some((bi, bj)) = found else {
        let len = count(&s, si) as i128;
        vp.prim_error(on_error, &format!("{}: index out of range: {} {} (length {})", name, i, j, len));
        return Ok(());
    };
    // A part of an ASCII String is ASCII
    let ri = if si == StrInfo::Ascii { StrInfo::Ascii } else { StrInfo::Unknown };
    vp.prim_return(vec![string_value_info(&s[bi..bj], ri)]);
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
    let si = info(&sp[0], &s);
    let b = if 0 <= i && i <= s.len() as i128 { char_to_byte_at(&sp[0], &s, i as usize, si) } else { None };
    let Some(b) = b else {
        let len = count(&s, si) as i128;
        vp.prim_error(on_error, &format!("{}: index out of range: {} (length {})", name, i, len));
        return Ok(());
    };
    let r = match s[b..].find(&*pattern) {
        None => Value::Null,
        Some(off) if si == StrInfo::Ascii => num((b + off) as f64),
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
    let si = info(&sp[0], &s);
    let b = if 0 <= i && i <= s.len() as i128 { char_to_byte_at(&sp[0], &s, i as usize, si) } else { None };
    let Some(b) = b else {
        let len = count(&s, si) as i128;
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
        // A part of an ASCII String is ASCII
        Some(t) if !suffix.is_empty() => {
            let ri = if known(&sp[0], &s) == StrInfo::Ascii { StrInfo::Ascii } else { StrInfo::Unknown };
            string_value_info(t, ri)
        }
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
        Some(c) => {
            let i = if c.is_ascii() { StrInfo::Ascii } else { StrInfo::Chars(1) };
            vp.prim_return(vec![string_value_info(&c.to_string(), i)])
        }
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
