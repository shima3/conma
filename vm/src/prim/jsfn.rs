//! __JS_fn_compile__ and __JS_fn_call__: functions of JavaScript that the host runs, called with
//! values of the VM copied to the host and back (stage H2 of docs/dev/CONJS-HOST-PLAN.md, 3.2 and 8).
//! The native VMs have no JavaScript: both go to onError.
//!
//! The values cross as JSON. To the host, a call is {"n":[node, ...],"a":[value, ...]}: the arguments,
//! and the nodes they reach. A value is a number ({"x":"NaN"}, {"x":"Infinity"}, {"x":"-Infinity"} or
//! {"x":"-0"} for those JSON cannot write), a string, true or false (the Booleans), null (null of ConMa),
//! {"@":i} (node i), {"l":[value, ...]} (an LList, copied as a list: it cannot be changed, so sharing it is
//! not kept) or {"h":k} (handle k: a value the host does not get, such as a Closure or a Stream; the host
//! can call a handle back, vm/wasm/src/lib.rs, conma_callback_handle). A node is {"d":[[key, value], ...]}
//! (a Dict whose keys are Strings, in its order) or {"r":value} (a Ref). The same Dict or Ref reached twice
//! is one node, so aliases and cycles are kept.
//!
//! The host answers {"s":status,"v":value,"w":[[i, node], ...],"n":[node, ...]}: the status "ok" (v is
//! the value returned), "throw" (v is what the function threw) or "escape" (v is a message: the function
//! came to something it cannot do), the nodes of the call it changed (w, written back into the Dicts and
//! Refs they came from), and the nodes it made (n, made here, and named {"+":j} in values). A status
//! "error" (v is a message) goes to onError.

use super::{take_n, PResult, VmError};
use crate::sched::Machine;
use crate::sys;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// The handles of the calls of __JS_fn_call__ that are running (the innermost last), for
    /// conma_callback_handle of the VM for wasm32.
    static HANDLES: RefCell<Vec<Vec<Value>>> = const { RefCell::new(Vec::new()) };
}

/// The value of handle k of the innermost call of __JS_fn_call__ that is running.
#[allow(dead_code)]
pub fn handle(k: usize) -> Option<Value> {
    HANDLES.with(|h| h.borrow().last().and_then(|l| l.get(k).cloned()))
}

/// __JS_fn_compile__ onError text ,(module): make the JavaScript text into a module of functions of
/// the host (a Number). The host evaluates text as the body of a function of rt (the runtime of the
/// host, vm/wasm/conma.js) that returns an object of functions.
pub fn compile(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__JS_fn_compile__";
    let mut a = take_n(vp, prim, 2)?;
    let text = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let text = match string_cow(&text) {
        Some(Ok(s)) => s.into_owned(),
        _ => {
            vp.prim_error(on_error, &format!("{}: expected String text, got {}", prim, value_type_name(&text)));
            return Ok(());
        }
    };
    match sys::jsfn_compile(&text) {
        Ok(id) => vp.prim_return1(num(id as f64)),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", prim, e)),
    }
    Ok(())
}

/// __JS_fn_call__ onError module name args ,(status value): call the function name of the module of
/// __JS_fn_compile__ with args (an LList), copied to the host; write back what it changed. status is
/// "ok", "throw" or "escape" (the header of this file).
pub fn call(m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__JS_fn_call__";
    let mut a = take_n(vp, prim, 4)?;
    let args = a.pop().unwrap();
    let name = a.pop().unwrap();
    let module = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Value::Number(id) = module else {
        vp.prim_error(on_error, &format!("{}: expected a Number module, got {}", prim, value_type_name(&module)));
        return Ok(());
    };
    let name = match string_cow(&name) {
        Some(Ok(s)) => s.into_owned(),
        _ => {
            vp.prim_error(on_error, &format!("{}: expected String name, got {}", prim, value_type_name(&name)));
            return Ok(());
        }
    };
    if !args.is_null() && !matches!(args, Value::LList(_)) {
        vp.prim_error(on_error, &format!("{}: expected an LList of arguments, got {}", prim, value_type_name(&args)));
        return Ok(());
    }
    let mut enc = Encoder::default();
    let mut out = String::from("{\"a\":[");
    let mut cur = args;
    let mut first = true;
    while let Value::LList(c) = cur {
        if !first {
            out.push(',');
        }
        first = false;
        enc.value(&c.head, &mut out);
        cur = c.tail.clone();
    }
    out.push_str("],\"n\":[");
    for (i, n) in enc.texts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(n);
    }
    out.push_str("]}");
    HANDLES.with(|h| h.borrow_mut().push(enc.handles.clone()));
    let answer = super::host::with_callback_context(m, vp, || sys::jsfn_call(id as i32, &name, &out));
    HANDLES.with(|h| h.borrow_mut().pop());
    let answer = match answer {
        Ok(a) => a,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: {}", prim, e));
            return Ok(());
        }
    };
    let json = match Json::parse(&answer) {
        Ok(j) => j,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: the host gave {}", prim, e));
            return Ok(());
        }
    };
    match back(m, &enc, &json) {
        Ok((status, value)) => {
            if status == "error" {
                let text = string_cow(&value).and_then(|r| r.ok()).map(|s| s.into_owned()).unwrap_or_default();
                vp.prim_error(on_error, &format!("{}: {}", prim, text));
            } else {
                vp.prim_return2(string_value(&status), value);
            }
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: the host gave {}", prim, e)),
    }
    Ok(())
}

/// The nodes and handles of a call, as they are written for the host.
#[derive(Default)]
struct Encoder {
    /// The Dicts and Refs by node, and their numbers by address.
    nodes: Vec<Value>,
    index: HashMap<usize, usize>,
    texts: Vec<String>,
    handles: Vec<Value>,
}

impl Encoder {
    fn handle(&mut self, v: &Value, out: &mut String) {
        out.push_str(&format!("{{\"h\":{}}}", self.handles.len()));
        self.handles.push(v.clone());
    }

    /// The node of the Dict or Ref v (made when it is first reached).
    fn node(&mut self, v: &Value, addr: usize, out: &mut String) {
        if let Some(i) = self.index.get(&addr) {
            out.push_str(&format!("{{\"@\":{}}}", i));
            return;
        }
        let i = self.nodes.len();
        self.index.insert(addr, i);
        self.nodes.push(v.clone());
        self.texts.push(String::new());
        out.push_str(&format!("{{\"@\":{}}}", i));
        let mut t = String::new();
        match v {
            Value::Dict(d) => {
                let entries = d.entries.borrow().clone();
                t.push_str("{\"d\":[");
                for (j, (k, x)) in entries.iter().enumerate() {
                    if j > 0 {
                        t.push(',');
                    }
                    t.push('[');
                    // a key that is not a String (not in Dicts of ConJS): the node keeps the others
                    match string_cow(k) {
                        Some(Ok(s)) => json_string(&s, &mut t),
                        _ => t.push_str("null"),
                    }
                    t.push(',');
                    self.value(x, &mut t);
                    t.push(']');
                }
                t.push_str("]}");
            }
            Value::Ref(r) => {
                let x = r.value.borrow().clone();
                t.push_str("{\"r\":");
                self.value(&x, &mut t);
                t.push('}');
            }
            _ => unreachable!(),
        }
        self.texts[i] = t;
    }

    fn value(&mut self, v: &Value, out: &mut String) {
        if v.is_null() {
            out.push_str("null");
            return;
        }
        if let Some(b) = super::control::boolean_choice(v) {
            out.push_str(if b { "true" } else { "false" });
            return;
        }
        match v {
            Value::Number(x) => json_number(*x, out),
            Value::Dict(d) => self.node(v, Rc::as_ptr(d) as usize, out),
            Value::Ref(r) if r.lazy.borrow().is_none() => self.node(v, Rc::as_ptr(r) as *const u8 as usize, out),
            Value::LList(_) => {
                out.push_str("{\"l\":[");
                let mut cur = v.clone();
                let mut first = true;
                while let Value::LList(c) = cur {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    self.value(&c.head, out);
                    cur = c.tail.clone();
                }
                out.push_str("]}");
                if !cur.is_null() {
                    // an improper LList does not happen; keep the rest as a handle
                    out.truncate(out.len() - 2);
                    out.push(',');
                    self.handle(&cur, out);
                    out.push_str("]}");
                }
            }
            _ => match string_cow(v) {
                Some(Ok(s)) => json_string(&s, out),
                _ => self.handle(v, out),
            },
        }
    }
}

fn json_number(x: f64, out: &mut String) {
    if x.is_nan() {
        out.push_str("{\"x\":\"NaN\"}");
    } else if x.is_infinite() {
        out.push_str(if x > 0.0 { "{\"x\":\"Infinity\"}" } else { "{\"x\":\"-Infinity\"}" });
    } else if x == 0.0 && x.is_sign_negative() {
        out.push_str("{\"x\":\"-0\"}");
    } else {
        out.push_str(&format!("{:?}", x));
    }
}

fn json_string(s: &str, out: &mut String) {
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

/// The answer of the host as values of the VM: the status and the value, after the changed nodes are
/// written back and the new ones made.
fn back(m: &mut Machine, enc: &Encoder, json: &Json) -> Result<(String, Value), String> {
    let field = |k: &str| json.get(k).ok_or_else(|| format!("no \"{}\"", k));
    let status = match field("s")? {
        Json::Str(s) => s.clone(),
        _ => return Err("a status that is not a string".to_string()),
    };
    // the new nodes first (empty), so that values can name them
    let empty = Vec::new();
    let made = match json.get("n") {
        Some(Json::Arr(a)) => a,
        _ => &empty,
    };
    let mut new_nodes = Vec::with_capacity(made.len());
    for n in made {
        if n.get("d").is_some() {
            m.serials.dicts += 1;
            new_nodes.push(Value::Dict(crate::gc::new_dict(DictObj::new(m.serials.dicts))));
        } else if n.get("r").is_some() {
            m.serials.refs += 1;
            new_nodes.push(Value::Ref(crate::gc::new_ref(RefObj::new(m.serials.refs, Value::Null))));
        } else {
            return Err("a new node that is neither a Dict nor a Ref".to_string());
        }
    }
    let ctx = Back { m, enc, new_nodes: &new_nodes };
    let ctx = &mut { ctx };
    let mut writes: Vec<(Value, &Json)> = made.iter().enumerate().map(|(j, n)| (new_nodes[j].clone(), n)).collect();
    if let Some(Json::Arr(w)) = json.get("w") {
        for pair in w {
            match pair {
                Json::Arr(p) if p.len() == 2 => match &p[0] {
                    Json::Num(i) => {
                        let node = enc.nodes.get(*i as usize).ok_or("a write of a node that is not there")?;
                        writes.push((node.clone(), &p[1]));
                    }
                    _ => return Err("a write without a node number".to_string()),
                },
                _ => return Err("a write that is not [node, contents]".to_string()),
            }
        }
    }
    let mut ref_changed = false;
    for (node, contents) in writes {
        match (&node, contents.get("d"), contents.get("r")) {
            (Value::Dict(d), Some(Json::Arr(entries)), _) => {
                let mut values = Vec::with_capacity(entries.len());
                for e in entries {
                    match e {
                        Json::Arr(kv) if kv.len() == 2 => match &kv[0] {
                            Json::Str(k) => values.push((string_value(k), ctx.value(&kv[1])?)),
                            _ => return Err("a key that is not a string".to_string()),
                        },
                        _ => return Err("an entry that is not [key, value]".to_string()),
                    }
                }
                while !d.entries.borrow().is_empty() {
                    let last = d.entries.borrow().len() - 1;
                    d.remove(last)?;
                }
                for (k, x) in values {
                    super::collections::dict_put_core(&node, k, x).map_err(|e| match e {
                        VmError::Error(s) => s,
                        VmError::Exit(_) => "exit".to_string(),
                    })?;
                }
            }
            (Value::Ref(r), _, Some(x)) => {
                let x = ctx.value(x)?;
                *r.value.borrow_mut() = x;
                ref_changed = true;
            }
            _ => return Err("a write that does not fit its node".to_string()),
        }
    }
    let value = ctx.value(field("v")?)?;
    if ref_changed {
        ctx.m.ref_changed = true;
    }
    Ok((status, value))
}

struct Back<'a> {
    m: &'a mut Machine,
    enc: &'a Encoder,
    new_nodes: &'a [Value],
}

impl Back<'_> {
    fn value(&self, j: &Json) -> Result<Value, String> {
        Ok(match j {
            Json::Null => Value::Null,
            Json::Bool(b) => {
                if *b {
                    self.m.true_closure()
                } else {
                    self.m.false_closure()
                }
            }
            Json::Num(x) => num(*x),
            Json::Str(s) => string_value(s),
            Json::Arr(_) => return Err("an array where a value goes".to_string()),
            Json::Obj(o) => match o.first() {
                Some((k, Json::Num(i))) if k == "@" => self.enc.nodes.get(*i as usize).cloned().ok_or("a node that is not there")?,
                Some((k, Json::Num(j))) if k == "+" => self.new_nodes.get(*j as usize).cloned().ok_or("a new node that is not there")?,
                Some((k, Json::Num(h))) if k == "h" => self.enc.handles.get(*h as usize).cloned().ok_or("a handle that is not there")?,
                Some((k, Json::Str(x))) if k == "x" => num(match x.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    "-0" => -0.0,
                    _ => return Err(format!("a number {}", x)),
                }),
                Some((k, Json::Arr(items))) if k == "l" => {
                    let mut tail = Value::Null;
                    for x in items.iter().rev() {
                        tail = Value::LList(Rc::new(LList { head: self.value(x)?, tail }));
                    }
                    tail
                }
                _ => return Err("an object that is not a value".to_string()),
            },
        })
    }
}

/// A JSON value, as the host writes it.
#[derive(Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn parse(text: &str) -> Result<Json, String> {
        let b = text.as_bytes();
        let mut i = 0;
        let v = parse_value(text, b, &mut i)?;
        skip_ws(b, &mut i);
        if i != b.len() {
            return Err("text after the JSON".to_string());
        }
        Ok(v)
    }
}

fn skip_ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && matches!(b[*i], b' ' | b'\n' | b'\r' | b'\t') {
        *i += 1;
    }
}

fn parse_value(text: &str, b: &[u8], i: &mut usize) -> Result<Json, String> {
    skip_ws(b, i);
    let Some(&c) = b.get(*i) else { return Err("the end of the JSON".to_string()) };
    match c {
        b'n' if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(Json::Null)
        }
        b't' if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(Json::Bool(true))
        }
        b'f' if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(Json::Bool(false))
        }
        b'"' => parse_string(text, b, i).map(Json::Str),
        b'[' => {
            *i += 1;
            let mut items = Vec::new();
            skip_ws(b, i);
            if b.get(*i) == Some(&b']') {
                *i += 1;
                return Ok(Json::Arr(items));
            }
            loop {
                items.push(parse_value(text, b, i)?);
                skip_ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b']') => {
                        *i += 1;
                        return Ok(Json::Arr(items));
                    }
                    _ => return Err("an array not closed".to_string()),
                }
            }
        }
        b'{' => {
            *i += 1;
            let mut items = Vec::new();
            skip_ws(b, i);
            if b.get(*i) == Some(&b'}') {
                *i += 1;
                return Ok(Json::Obj(items));
            }
            loop {
                skip_ws(b, i);
                if b.get(*i) != Some(&b'"') {
                    return Err("a key that is not a string".to_string());
                }
                let k = parse_string(text, b, i)?;
                skip_ws(b, i);
                if b.get(*i) != Some(&b':') {
                    return Err("no : after a key".to_string());
                }
                *i += 1;
                items.push((k, parse_value(text, b, i)?));
                skip_ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b'}') => {
                        *i += 1;
                        return Ok(Json::Obj(items));
                    }
                    _ => return Err("an object not closed".to_string()),
                }
            }
        }
        _ => {
            let start = *i;
            while *i < b.len() && matches!(b[*i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                *i += 1;
            }
            text[start..*i].parse::<f64>().map(Json::Num).map_err(|_| format!("a value at byte {}", start))
        }
    }
}

fn parse_string(text: &str, b: &[u8], i: &mut usize) -> Result<String, String> {
    *i += 1;
    let mut s = String::new();
    let mut start = *i;
    loop {
        let Some(&c) = b.get(*i) else { return Err("a string not closed".to_string()) };
        match c {
            b'"' => {
                s.push_str(&text[start..*i]);
                *i += 1;
                return Ok(s);
            }
            b'\\' => {
                s.push_str(&text[start..*i]);
                let e = *b.get(*i + 1).ok_or("a string not closed")?;
                *i += 2;
                match e {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    b'b' => s.push('\u{8}'),
                    b'f' => s.push('\u{c}'),
                    b'u' => {
                        let hex = |at: usize| text.get(at..at + 4).and_then(|h| u32::from_str_radix(h, 16).ok());
                        let mut code = hex(*i).ok_or("a bad \\u")?;
                        *i += 4;
                        if (0xD800..0xDC00).contains(&code) && text.get(*i..*i + 2) == Some("\\u") {
                            if let Some(low) = hex(*i + 2) {
                                if (0xDC00..0xE000).contains(&low) {
                                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                    *i += 6;
                                }
                            }
                        }
                        s.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                    }
                    _ => return Err("a bad escape".to_string()),
                }
                start = *i;
            }
            _ => *i += 1,
        }
    }
}
