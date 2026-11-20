//! __Host_call__ onError name args ,(result): call the function of the host
//! named name (docs/dev/PLATFORM-PLAN-ja.md 3.2, stage P2). The values that go
//! to the host and come back are Numbers, Strings and LLists of them (null is
//! the empty LList); Dicts and Booleans are not passed yet. The functions of
//! BUILTIN are in every VM built from this crate (the native, transpiled and
//! wasm VMs; vm/python/vproc_step.py has the same), so that tests run the same
//! everywhere; the VM for wasm32 also asks its host for the other names
//! (sys::host_call, the hook hostFunctions of vm/wasm/conma.js).
//!
//! __Host_start__, __Host_start_tracked__ and __Host_result__ are the calls
//! that wait (3.4 of the plan, stage P3): the result comes as the JSON text of
//! a result ({"r":value} or {"e":"message"}) in a pipe, whose read end the
//! program waits for with __OS_wait_readable__ (the pipe is at its end when
//! the call has finished). The functions of BUILTIN finish at once; test.later
//! finishes by the virtual clock of Later, the same on every VM; on the VM for
//! wasm32 the other names go to the host (sys::host_start), which writes the
//! result to the pipe when the function (maybe a Promise) has finished.

use super::stream::{check_stream, new_stream};
use super::{take_n, PResult, VmError};
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
/// for the tests (test/fast14, test/fast15). test.later, which waits, is not
/// here: __Host_start__ handles it (Later); __Host_call__ has no such name.
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

/// The functions of the host of the feature "time" (docs/dev/PLATFORM-PLAN-ja.md 4.4, stage P5) that
/// the native and transpiled VMs have (vm/python/vproc_step.py has the same); the VM for wasm32,
/// which has no clock, asks its host (vm/wasm/conma.js). time.sleep, which waits, is not here:
/// __Host_start__ handles it.
#[cfg(unix)]
const NATIVE: &[(&str, HostFn)] = &[("time.now", time_now)];
#[cfg(not(unix))]
const NATIVE: &[(&str, HostFn)] = &[];

/// time.now: the time in milliseconds since 1970-01-01 00:00 UTC.
#[cfg(unix)]
fn time_now(_args: &[HostValue]) -> Result<HostValue, String> {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| "time.now: the clock is before 1970")?;
    Ok(HostValue::Num(d.as_secs_f64() * 1000.0))
}

/// The function of BUILTIN or NATIVE named name.
fn builtin(name: &str) -> Option<HostFn> {
    BUILTIN.iter().chain(NATIVE.iter()).find(|(n, _)| *n == name).map(|(_, f)| *f)
}

/// time.sleep ms waits on the native and transpiled VMs (on the VM for wasm32 the host does it).
const SLEEP: &str = "time.sleep";

/// The calls of time.sleep that have not finished, by the real clock. The scheduler finishes those
/// whose time has come between two steps (sched.rs), so that a VProc sees the result written and the
/// pipe closed at once (a thread writing them could be seen between the two), and sleeps in poll()
/// no longer than until the next one.
#[derive(Default)]
pub struct Timers {
    /// (when, write end of the pipe, result text)
    jobs: Vec<(std::time::Instant, i32, String)>,
}

impl Timers {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn add(&mut self, ms: f64, w: i32, text: String) {
        let d = std::time::Duration::from_secs_f64(ms.min(1e12) / 1000.0);
        self.jobs.push((std::time::Instant::now() + d, w, text));
    }

    /// Finish the calls whose time has come.
    pub fn finish_due(&mut self) {
        let now = std::time::Instant::now();
        let mut i = 0;
        while i < self.jobs.len() {
            if self.jobs[i].0 <= now {
                let (_, w, text) = self.jobs.remove(i);
                finish(w, &text);
            } else {
                i += 1;
            }
        }
    }

    /// The milliseconds until the next call is due (rounded up), or None if there is none.
    pub fn timeout_ms(&self) -> Option<i32> {
        let now = std::time::Instant::now();
        self.jobs
            .iter()
            .map(|(t, _, _)| t.saturating_duration_since(now))
            .min()
            .map(|d| d.as_micros().div_ceil(1000).min(i32::MAX as u128) as i32)
    }
}

/// The platform features that the functions of the host give: "test" on every
/// VM, "time" on the native and transpiled VMs, and on the VM for wasm32 those its host names
/// (sys::host_feature: "time" too).
pub fn feature(name: &str) -> bool {
    name == "test" || (cfg!(unix) && name == "time") || sys::host_feature(name)
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

/// The name and the arguments of a call of the host (the arguments a, after
/// onError, of __Host_call__ and __Host_start__); None after going to onError.
fn name_args(vp: &mut VProc, prim: &str, on_error: &Value, name: &Value, args: &Value) -> Option<(String, Vec<HostValue>)> {
    let Some(Ok(fname)) = string_cow(name) else {
        vp.prim_error(on_error.clone(), &format!("{}: expected String name, got {}", prim, value_type_name(name)));
        return None;
    };
    match to_host(args) {
        Ok(HostValue::List(items)) => Some((fname.into_owned(), items)),
        Ok(_) => {
            vp.prim_error(on_error.clone(), &format!("{}: expected an LList of Numbers, Strings and LLists, got {}", prim, value_type_name(args)));
            None
        }
        Err(t) => {
            vp.prim_error(on_error.clone(), &format!("{}: expected an LList of Numbers, Strings and LLists, got {}", prim, t));
            None
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
    let Some((fname, args)) = name_args(vp, prim, &on_error, &name, &args) else { return Ok(()) };
    let result = match builtin(&fname) {
        Some(f) => f(&args),
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

/// The JSON text of a result.
fn result_json(r: &Result<HostValue, String>) -> String {
    match r {
        Ok(h) => format!("{{\"r\":{}}}", to_json(h)),
        Err(message) => format!("{{\"e\":{}}}", to_json(&HostValue::Str(message.clone()))),
    }
}

/// Write the result text to the pipe end w and close it. A reader that has
/// gone (its VProc was killed and the Stream closed) makes the write fail:
/// the result is dropped then.
fn finish(w: i32, text: &str) {
    let mut b = text.as_bytes();
    while !b.is_empty() {
        match sys::write_fd(w, b) {
            Ok(0) | Err(_) => break,
            Ok(n) => b = &b[n..],
        }
    }
    let _ = sys::close_fd(w);
}

/// The longest result that a function of BUILTIN or test.later writes to its
/// pipe: a pipe of the OS takes at least this much without a reader (the
/// native and Python VMs write it in the step of the call, or in the
/// scheduler for test.later, before any VProc reads it).
const PIPE_MAX: usize = 8192;

/// The result text, or an error result if it is longer than PIPE_MAX (the
/// scheduler writes the result of test.later, so a longer one would block it).
fn capped(prim: &str, fname: &str, text: String) -> String {
    if text.len() > PIPE_MAX {
        result_json(&Err(format!("{}: the result of {} is longer than {} bytes", prim, fname, PIPE_MAX)))
    } else {
        text
    }
}

/// The calls of test.later that have not finished, by the virtual clock: when
/// no VProc can run, the call with the earliest deadline (then the first
/// called) finishes, and the clock moves to its deadline (3.4 of the plan).
#[derive(Default)]
pub struct Later {
    clock: f64,
    calls: u64,
    /// (deadline, number of the call, write end of the pipe, result text)
    jobs: Vec<(f64, u64, i32, String)>,
}

impl Later {
    /// Finish the next call of test.later; false if there is none.
    pub fn finish_next(&mut self) -> bool {
        let Some(i) = (0..self.jobs.len()).min_by(|&a, &b| {
            let (x, y) = (&self.jobs[a], &self.jobs[b]);
            x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal).then(x.1.cmp(&y.1))
        }) else {
            return false;
        };
        let (deadline, _, w, text) = self.jobs.remove(i);
        self.clock = deadline;
        finish(w, &text);
        true
    }
}

/// __Host_start__ onError name args ,(inputStream)
pub fn start(m: &mut Machine, vp: &mut VProc) -> PResult {
    start_with(m, vp, "__Host_start__", false)
}

/// __Host_start_tracked__ onError name args Ref ,(inputStream): __Host_start__,
/// and in the same step, put the Stream in front of the LList in the Ref (as
/// __OS_open_tracked__), so that a killed VProc leaves no pipe open.
pub fn start_tracked(m: &mut Machine, vp: &mut VProc) -> PResult {
    start_with(m, vp, "__Host_start_tracked__", true)
}

fn start_with(m: &mut Machine, vp: &mut VProc, prim: &str, tracked: bool) -> PResult {
    let mut a = take_n(vp, prim, if tracked { 4 } else { 3 })?;
    let track = if tracked { a.pop() } else { None };
    let args = a.pop().unwrap();
    let name = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let track = match track {
        None => None,
        Some(Value::Ref(r)) if matches!(&*r.value.borrow(), Value::Null | Value::LList(_)) => Some(r),
        Some(v) => {
            let got = match &v {
                Value::Ref(r) => format!("a Ref of {}", value_type_name(&r.value.borrow())),
                other => value_type_name(other).to_string(),
            };
            vp.prim_error(on_error, &format!("{}: expected a Ref of an LList, got {}", prim, got));
            return Ok(());
        }
    };
    let Some((fname, args)) = name_args(vp, prim, &on_error, &name, &args) else { return Ok(()) };
    let builtin = builtin(&fname);
    let later = fname == "test.later";
    let sleep = cfg!(unix) && fname == SLEEP;
    if sleep {
        match args.first() {
            Some(HostValue::Num(n)) if *n >= 0.0 => {}
            _ => {
                vp.prim_error(on_error, &format!("{}: time.sleep: expected milliseconds (a Number, 0 or more)", prim));
                return Ok(());
            }
        }
    } else if later {
        match args.first() {
            Some(HostValue::Num(n)) if *n >= 0.0 => {}
            _ => {
                vp.prim_error(on_error, &format!("{}: test.later: expected a count (a Number, 0 or more) first", prim));
                return Ok(());
            }
        }
    } else if builtin.is_none() && !sys::host_has(&fname) {
        vp.prim_error(on_error, &format!("{}: no host function \"{}\"", prim, fname));
        return Ok(());
    }
    let (r, w) = sys::make_pipe().map_err(|e| VmError::Error(format!("{}: {}", prim, sys::errno_str(&e))))?;
    if later {
        let deadline = m.host_later.clock + if let Some(HostValue::Num(n)) = args.first() { *n } else { 0.0 };
        m.host_later.calls += 1;
        let n = m.host_later.calls;
        let text = capped(prim, &fname, result_json(&Ok(HostValue::List(args))));
        m.host_later.jobs.push((deadline, n, w, text));
    } else if sleep {
        // the scheduler finishes it when the time has come (Timers)
        let ms = if let Some(HostValue::Num(n)) = args.first() { *n } else { 0.0 };
        m.host_timers.add(ms, w, result_json(&Ok(HostValue::List(Vec::new()))));
    } else if let Some(f) = builtin {
        finish(w, &capped(prim, &fname, result_json(&f(&args))));
    } else {
        sys::host_start(&fname, &to_json(&HostValue::List(args)), w);
    }
    let s = Value::Stream(new_stream(m, r, 'r'));
    if let Some(t) = track {
        let tail = std::mem::replace(&mut *t.value.borrow_mut(), Value::Null);
        *t.value.borrow_mut() = Value::LList(Rc::new(LList { head: s.clone(), tail }));
        m.ref_changed = true;
    }
    vp.prim_return1(s);
    Ok(())
}

/// __Host_result__ onError inputStream ,(result): the result of the call of
/// __Host_start__ that gave inputStream, which is then closed. onError if the
/// call failed, or has not finished (the pipe is not at its end).
pub fn result(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__Host_result__";
    let mut a = take_n(vp, prim, 2)?;
    let v = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Some(s) = check_stream(vp, &on_error, prim, &v, Some('r')) else { return Ok(()) };
    let mut text = Vec::new();
    let mut buf = [0u8; 4096];
    let ended = loop {
        match sys::readable(s.fd).and_then(|ok| if ok { sys::read_fd(s.fd, &mut buf).map(Some) } else { Ok(None) }) {
            Ok(Some(0)) => break Ok(true),
            Ok(Some(n)) => text.extend_from_slice(&buf[..n]),
            Ok(None) => break Ok(false),
            Err(e) => break Err(e),
        }
    };
    let r = match ended {
        Err(e) => Err(format!("{}: {}", prim, sys::errno_str(&e))),
        Ok(false) => Err(format!("{}: the call has not finished", prim)),
        Ok(true) => {
            let _ = s.close();
            match from_json(&String::from_utf8_lossy(&text)) {
                Ok(r) => r,
                Err(e) => Err(format!("{}: the host gave {}", prim, e)),
            }
        }
    };
    match r {
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
