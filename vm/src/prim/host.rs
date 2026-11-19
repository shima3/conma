//! __Host_call__ onError name args ,(result): call the function of the host
//! named name (docs/dev/PLATFORM-PLAN-ja.md 3.2, stage P2). The values that go
//! to the host and come back are Numbers, Strings and LLists of them (null is
//! the empty LList); Dicts and Booleans are not passed yet. The functions of
//! BUILTIN are in every VM built from this crate (the native, transpiled and
//! wasm VMs; vm/python/vproc_step.py has the same), so that tests run the same
//! everywhere; the VM for wasm32 also asks its host for the other names
//! (sys::host_call, the hook hostFunctions of vm/wasm/conma.js).

use super::{take_n, PResult};
use crate::sched::Machine;
use crate::sys;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// A value that goes to the host or comes back.
#[derive(Clone, Debug)]
pub enum HostValue {
    Num(f64),
    Str(String),
    /// An LList; the empty one is null.
    List(Vec<HostValue>),
}

type HostFn = fn(&[HostValue]) -> Result<HostValue, String>;

/// The functions of the host that every VM has: those of the feature "test",
/// for the tests (test/fast14).
const BUILTIN: &[(&str, HostFn)] = &[("test.add", test_add), ("test.echo", test_echo), ("test.fail", test_fail)];

/// test.add: the sum of its arguments, Numbers, from the left (0 for none).
fn test_add(args: &[HostValue]) -> Result<HostValue, String> {
    let mut sum = 0.0;
    for a in args {
        match a {
            HostValue::Num(x) => sum += x,
            _ => return Err("test.add: expected Numbers".into()),
        }
    }
    Ok(HostValue::Num(sum))
}

/// test.echo: its arguments, as an LList.
fn test_echo(args: &[HostValue]) -> Result<HostValue, String> {
    Ok(HostValue::List(args.to_vec()))
}

/// test.fail: always fails.
fn test_fail(_args: &[HostValue]) -> Result<HostValue, String> {
    Err("test.fail: failed".into())
}

/// The platform features that the functions of the host give: "test" on every
/// VM, and on the VM for wasm32 those its host names (sys::host_feature).
pub fn feature(name: &str) -> bool {
    name == "test" || sys::host_feature(name)
}

/// The value v as a value for the host; Err with the type name of the first
/// value that is not a Number, a String or an LList of them.
fn to_host(v: &Value) -> Result<HostValue, String> {
    if v.is_null() {
        return Ok(HostValue::List(Vec::new()));
    }
    match v {
        Value::Number(x) => Ok(HostValue::Num(*x)),
        Value::LList(_) => {
            let mut items = Vec::new();
            let mut cur = v.clone();
            loop {
                if cur.is_null() {
                    break;
                }
                let next = match &cur {
                    Value::LList(c) => {
                        items.push(to_host(&c.head)?);
                        c.tail.clone()
                    }
                    other => return Err(value_type_name(other)),
                };
                cur = next;
            }
            Ok(HostValue::List(items))
        }
        _ => match string_cow(v) {
            Some(Ok(s)) => Ok(HostValue::Str(s.into_owned())),
            Some(Err(e)) => Err(e),
            None => Err(value_type_name(v)),
        },
    }
}

/// A value of the host as a value of ConMa.
fn from_host(h: &HostValue) -> Value {
    match h {
        HostValue::Num(x) => num(*x),
        HostValue::Str(s) => string_value(s),
        HostValue::List(items) => {
            items.iter().rev().fold(Value::Null, |tail, head| Value::LList(Rc::new(LList { head: from_host(head), tail })))
        }
    }
}

/// __Host_call__ onError name args ,(result)
pub fn call(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__Host_call__";
    let mut a = take_n(vp, prim, 3)?;
    let args = a.pop().unwrap();
    let name = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Some(Ok(fname)) = string_cow(&name) else {
        vp.prim_error(on_error, &format!("{}: expected String name, got {}", prim, value_type_name(&name)));
        return Ok(());
    };
    let fname = fname.into_owned();
    let args = match to_host(&args) {
        Ok(HostValue::List(items)) => items,
        Ok(_) => {
            vp.prim_error(on_error, &format!("{}: expected an LList of Numbers, Strings and LLists, got {}", prim, value_type_name(&args)));
            return Ok(());
        }
        Err(t) => {
            vp.prim_error(on_error, &format!("{}: expected an LList of Numbers, Strings and LLists, got {}", prim, t));
            return Ok(());
        }
    };
    let result = match BUILTIN.iter().find(|(n, _)| *n == fname) {
        Some((_, f)) => f(&args),
        None => match sys::host_call(&fname, &to_json(&HostValue::List(args))) {
            None => Err(format!("{}: no host function \"{}\"", prim, fname)),
            Some(Err(message)) => Err(message),
            Some(Ok(json)) => match from_json(&json) {
                Ok(r) => r,
                Err(e) => Err(format!("{}: {} returned {}", prim, fname, e)),
            },
        },
    };
    match result {
        Ok(h) => vp.prim_return1(from_host(&h)),
        Err(message) => vp.prim_error(on_error, &message),
    }
    Ok(())
}

// The values go to the host of the VM for wasm32 and come back as JSON: a
// Number that JSON cannot write (-0, NaN, an infinity) as {"n":"-0"},
// {"n":"NaN"}, {"n":"Infinity"} or {"n":"-Infinity"}, null for the empty LList.
// A result is {"r":value} or {"e":"message"} (vm/wasm/conma.js, hostCall).

/// The JSON of a value for the host.
pub fn to_json(h: &HostValue) -> String {
    let mut out = String::new();
    write_json(h, &mut out);
    out
}

fn write_json(h: &HostValue, out: &mut String) {
    match h {
        HostValue::Num(x) => {
            if x.is_nan() {
                out.push_str("{\"n\":\"NaN\"}");
            } else if x.is_infinite() {
                out.push_str(if *x > 0.0 { "{\"n\":\"Infinity\"}" } else { "{\"n\":\"-Infinity\"}" });
            } else if *x == 0.0 && x.is_sign_negative() {
                out.push_str("{\"n\":\"-0\"}");
            } else {
                // Rust writes the shortest digits that read back as the same f64, as JSON needs
                out.push_str(&format!("{:?}", x));
            }
        }
        HostValue::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        HostValue::List(items) if items.is_empty() => out.push_str("null"),
        HostValue::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
    }
}

/// A result of the host: Ok(Ok(value)) for {"r":value}, Ok(Err(message)) for
/// {"e":"message"}, and Err for a text that is neither.
pub fn from_json(text: &str) -> Result<Result<HostValue, String>, String> {
    let mut p = Json { s: text.as_bytes(), i: 0 };
    p.ws();
    p.expect(b'{')?;
    p.ws();
    let key = p.string()?;
    p.ws();
    p.expect(b':')?;
    p.ws();
    let r = match key.as_str() {
        "r" => Ok(p.value()?),
        "e" => Err(p.string()?),
        _ => return Err(format!("an object with the key \"{}\"", key)),
    };
    p.ws();
    p.expect(b'}')?;
    p.ws();
    if p.i != p.s.len() {
        return Err("more than one value".into());
    }
    Ok(r)
}

struct Json<'a> {
    s: &'a [u8],
    i: usize,
}

impl Json<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("a JSON text that is not a result (at byte {})", self.i))
        }
    }

    fn value(&mut self) -> Result<HostValue, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'"') => Ok(HostValue::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(HostValue::List(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(HostValue::List(items));
                        }
                        _ => return Err(format!("a JSON text that is not a result (at byte {})", self.i)),
                    }
                }
            }
            Some(b'n') if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(HostValue::List(Vec::new()))
            }
            Some(b'{') => {
                self.i += 1;
                self.ws();
                let key = self.string()?;
                self.ws();
                self.expect(b':')?;
                self.ws();
                let v = self.string()?;
                self.ws();
                self.expect(b'}')?;
                let x = match (key.as_str(), v.as_str()) {
                    ("n", "-0") => -0.0,
                    ("n", "NaN") => f64::NAN,
                    ("n", "Infinity") => f64::INFINITY,
                    ("n", "-Infinity") => f64::NEG_INFINITY,
                    _ => return Err("a value that is not a Number, a String or a list".into()),
                };
                Ok(HostValue::Num(x))
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[start..self.i]).unwrap();
                t.parse::<f64>().map(HostValue::Num).map_err(|_| format!("the number {}", t))
            }
            _ => Err("a value that is not a Number, a String or a list".into()),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else { return Err("a JSON string without its end".into()) };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else { return Err("a JSON string without its end".into()) };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let low = self.hex4()?;
                                code = 0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err("a JSON string with a wrong escape".into()),
                    }
                }
                c => out.push(c),
            }
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let t = self.s.get(self.i..self.i + 4).ok_or("a JSON string with a wrong escape")?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(t).map_err(|_| "a JSON string with a wrong escape")?, 16)
            .map_err(|_| "a JSON string with a wrong escape".into())
    }
}
