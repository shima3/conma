//! __JS_fn_compile__, __JS_fn_call__ and __JS_fn_resume__: functions of JavaScript that the host runs,
//! called with values of the VM copied to the host and back (stages H2a and H2b of
//! docs/dev/CONJS-HOST-PLAN.md, 3.1, 3.2 and 8). The native VMs have no JavaScript: they go to onError.
//!
//! **A session.** A call keeps, on both sides, a table of the nodes it has copied: the Dicts and Refs of
//! the VM, each copied once, so that the host sees the same object for the same Dict and keeps cycles.
//! Both sides add nodes at the end of the table in the same order, so a node has the same number on both.
//! The VM keeps, for each node, what it was when the host last saw it (a Dict shares its entries, which
//! a change copies first, so a change is seen as other entries; a Ref keeps its value), and sends again
//! only the nodes that changed. The host does the same with its objects. The session lives until the
//! function ends; a function that yields (a generator) keeps it until it is resumed and ends.
//!
//! **The JSON.** A value is a number ({"x":"NaN"}, {"x":"Infinity"}, {"x":"-Infinity"} or {"x":"-0"}
//! for those JSON cannot write), a string, true or false (the Booleans), null (null of ConMa), {"@":i}
//! (node i), {"l":[value, ...]} (an LList, copied as a list: it cannot change, so sharing it is not
//! kept) or {"h":k} (handle k: a value the host does not get, such as a Closure or a Stream, which the
//! host can call back). A node is {"d":[[key, value], ...]} (a Dict whose keys are Strings, in its order)
//! or {"r":value} (a Ref). Each message carries "n", the contents of the nodes it adds (numbered from
//! the size of the table before it), and "w", [[i, node], ...], the new contents of nodes that changed.
//!
//! * The VM calls with {"a":[value, ...],"w":[],"n":[...]}, and resumes with {"how":"next" or "throw",
//!   "v":value,"w":[...],"n":[...]}.
//! * The host answers {"s":status,"v":value,"w":[...],"n":[...]}: "ok" (v returned), "throw" (v thrown),
//!   "escape" (v is a message: the function came to something it cannot do; nothing of the answer is
//!   written back), "yield" (a generator yielded v, a request, and waits to be resumed; "g" is its
//!   number), or "error" (v is a message for onError).
//! * Values by reference (stage S0 of docs/dev/CONJS-SHELL-PLAN.md, section 7): {"o":k} is object k of
//!   the table of the host, which the VM holds as a Value::Host (one HostObj for each k; when the VM drops
//!   it, the host forgets k: host_obj_free), and {"k":i} is value i that the host keeps (KEPT), which
//!   stays the same value of the VM across calls. The host keeps a value of a call by the call back
//!   {"keep":value,"w":[...],"n":[...]}, answered {"r":i,...}, and lets it go with conma_jsfn_release.
//! * During a call, the host calls a value of the VM back with {"f":value,"a":[...],"w":[...],"n":[...]}
//!   (vm/wasm/src/lib.rs, conma_jsfn_callback), after sending what it changed; the VM answers {"r":value,
//!   "w":[...],"n":[...]}, with what the call back changed, {"e":message} or {"x":message} (an escape).

use super::{take_n, PResult, VmError};
use crate::sched::Machine;
use crate::sys;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// The sessions of the calls and resumptions running (the innermost last).
    static SESSIONS: RefCell<Vec<Rc<RefCell<Session>>>> = const { RefCell::new(Vec::new()) };
    /// The sessions of the generators waiting to be resumed, by number.
    static GENERATORS: RefCell<HashMap<i64, Rc<RefCell<Session>>>> = RefCell::new(HashMap::new());
    /// The objects of the host that the VM holds, by their numbers in the host (Value::Host).
    static HOST_OBJS: RefCell<HashMap<i32, std::rc::Weak<HostObj>>> = RefCell::new(HashMap::new());
    /// The values of the VM that the host keeps across calls, by number, and the next number.
    static KEPT: RefCell<(HashMap<i64, Value>, i64)> = RefCell::new((HashMap::new(), 0));
}

/// The VM dropped its last reference to object k of the host: the host forgets it.
pub fn host_obj_dropped(k: i32) {
    // try_with: nothing to do when the table has gone (the VM is ending)
    let _ = HOST_OBJS.try_with(|t| t.borrow_mut().remove(&k));
    sys::host_obj_free(k);
}

/// The value of the VM for object k of the host: the same HostObj while the VM holds it.
pub fn host_obj(k: i32) -> Value {
    HOST_OBJS.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(h) = t.get(&k).and_then(|w| w.upgrade()) {
            return Value::Host(h);
        }
        let h = Rc::new(HostObj { host: k });
        t.insert(k, Rc::downgrade(&h));
        Value::Host(h)
    })
}

/// The host let go of value i of the VM that it kept (conma_jsfn_release of vm/wasm/src/lib.rs).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn release(i: i64) {
    let v = KEPT.with(|k| k.borrow_mut().0.remove(&i));
    drop(v);
}

/// The number of values of the VM that the host keeps (for the tests).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn kept_count() -> usize {
    KEPT.with(|k| k.borrow().0.len())
}

/// What a node was when the other side last saw it.
enum Snap {
    Dict(Rc<Vec<(Value, Value)>>),
    Ref(Value),
}

/// The nodes and handles of a call (the header of this file).
#[derive(Default)]
struct Session {
    nodes: Vec<Value>,
    index: HashMap<usize, usize>,
    snaps: Vec<Option<Snap>>,
    handles: Vec<Value>,
    /// The Booleans of the Machine of the call.
    booleans: Option<(Value, Value)>,
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.to_bits() == y.to_bits(),
        _ => same(a, b),
    }
}

fn address(v: &Value) -> usize {
    match v {
        Value::Dict(d) => Rc::as_ptr(d) as *const u8 as usize,
        Value::Ref(r) => Rc::as_ptr(r) as *const u8 as usize,
        _ => 0,
    }
}

impl Session {
    /// Write the value v; the nodes it adds go to made (their contents, in order).
    fn value(&mut self, v: &Value, out: &mut String, made: &mut Vec<String>) {
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
            Value::Host(h) => out.push_str(&format!("{{\"o\":{}}}", h.host)),
            Value::Dict(_) => self.node(v, out, made),
            Value::Ref(r) if r.lazy.borrow().is_none() => self.node(v, out, made),
            Value::LList(_) => {
                out.push_str("{\"l\":[");
                let mut cur = v.clone();
                let mut first = true;
                while let Value::LList(c) = cur {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    self.value(&c.head, out, made);
                    cur = c.tail.clone();
                }
                if !cur.is_null() {
                    // an improper LList (not made by ConJS): its rest as a handle
                    if !first {
                        out.push(',');
                    }
                    self.handle(&cur, out);
                }
                out.push_str("]}");
            }
            _ => match string_cow(v) {
                Some(Ok(s)) => json_string(&s, out),
                _ => self.handle(v, out),
            },
        }
    }

    fn handle(&mut self, v: &Value, out: &mut String) {
        out.push_str(&format!("{{\"h\":{}}}", self.handles.len()));
        self.handles.push(v.clone());
    }

    /// The node of the Dict or Ref v, added (with its contents in made) when it is first reached.
    fn node(&mut self, v: &Value, out: &mut String, made: &mut Vec<String>) {
        let addr = address(v);
        if let Some(i) = self.index.get(&addr) {
            out.push_str(&format!("{{\"@\":{}}}", i));
            return;
        }
        let i = self.nodes.len();
        self.index.insert(addr, i);
        self.nodes.push(v.clone());
        self.snaps.push(None);
        out.push_str(&format!("{{\"@\":{}}}", i));
        let at = made.len();
        made.push(String::new());
        made[at] = self.contents(i, made);
    }

    /// The contents of node i, which the other side now has.
    fn contents(&mut self, i: usize, made: &mut Vec<String>) -> String {
        let mut t = String::new();
        match self.nodes[i].clone() {
            Value::Dict(d) => {
                let entries = d.entries.borrow().clone();
                self.snaps[i] = Some(Snap::Dict(entries.clone()));
                t.push_str("{\"d\":[");
                for (j, (k, x)) in entries.iter().enumerate() {
                    if j > 0 {
                        t.push(',');
                    }
                    t.push('[');
                    // a key that is not a String (not in the Dicts of ConJS)
                    match string_cow(k) {
                        Some(Ok(s)) => json_string(&s, &mut t),
                        _ => t.push_str("null"),
                    }
                    t.push(',');
                    self.value(x, &mut t, made);
                    t.push(']');
                }
                t.push_str("]}");
            }
            Value::Ref(r) => {
                let x = r.value.borrow().clone();
                self.snaps[i] = Some(Snap::Ref(x.clone()));
                t.push_str("{\"r\":");
                self.value(&x, &mut t, made);
                t.push('}');
            }
            _ => unreachable!(),
        }
        t
    }

    /// The nodes below upto that changed since the other side saw them, with their contents.
    fn changes(&mut self, upto: usize, made: &mut Vec<String>) -> String {
        let mut w = String::from("[");
        for i in 0..upto {
            let changed = match (&self.nodes[i], &self.snaps[i]) {
                (Value::Dict(d), Some(Snap::Dict(s))) => !Rc::ptr_eq(&d.entries.borrow(), s),
                (Value::Ref(r), Some(Snap::Ref(s))) => !same_value(&r.value.borrow(), s),
                _ => false,
            };
            if changed {
                if w.len() > 1 {
                    w.push(',');
                }
                let c = self.contents(i, made);
                w.push_str(&format!("[{},{}]", i, c));
            }
        }
        w.push(']');
        w
    }

    /// A message to the host: head (its first fields), then "w" (the changes of the nodes the host
    /// had) and "n" (the nodes added).
    fn message(&mut self, head: impl FnOnce(&mut Session, &mut String, &mut Vec<String>)) -> String {
        let had = self.nodes.len();
        let mut made = Vec::new();
        let w = self.changes(had, &mut made);
        let mut out = String::from("{");
        head(self, &mut out, &mut made);
        out.push_str(",\"w\":");
        out.push_str(&w);
        out.push_str(",\"n\":[");
        out.push_str(&made.join(","));
        out.push_str("]}");
        out
    }

    /// Take in what the host sends ("n", then "w"): the nodes it made become Dicts and Refs, and the
    /// nodes it changed are written back.
    fn take(&mut self, mk: &mut MakeValues, json: &Json) -> Result<(), String> {
        let base = self.nodes.len();
        let empty = Vec::new();
        let made = match json.get("n") {
            Some(Json::Arr(a)) => a,
            _ => &empty,
        };
        for n in made {
            let v = if n.get("d").is_some() {
                mk.dicts += 1;
                Value::Dict(crate::gc::new_dict(DictObj::new(mk.dicts)))
            } else if n.get("r").is_some() {
                mk.refs += 1;
                Value::Ref(crate::gc::new_ref(RefObj::new(mk.refs, Value::Null)))
            } else {
                return Err("a new node that is neither a Dict nor a Ref".to_string());
            };
            self.index.insert(address(&v), self.nodes.len());
            self.nodes.push(v);
            self.snaps.push(None);
        }
        let mut writes: Vec<(usize, &Json)> = made.iter().enumerate().map(|(j, n)| (base + j, n)).collect();
        if let Some(Json::Arr(w)) = json.get("w") {
            for pair in w {
                match pair {
                    Json::Arr(p) if p.len() == 2 => match &p[0] {
                        Json::Num(i) if (*i as usize) < base => writes.push((*i as usize, &p[1])),
                        _ => return Err("a write of a node that is not there".to_string()),
                    },
                    _ => return Err("a write that is not [node, contents]".to_string()),
                }
            }
        }
        for (i, contents) in writes {
            let node = self.nodes[i].clone();
            match (&node, contents.get("d"), contents.get("r")) {
                (Value::Dict(d), Some(Json::Arr(entries)), _) => {
                    let mut values = Vec::with_capacity(entries.len());
                    for e in entries {
                        match e {
                            Json::Arr(kv) if kv.len() == 2 => match &kv[0] {
                                Json::Str(k) => values.push((string_value(k), self.decode(mk, &kv[1])?)),
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
                    self.snaps[i] = Some(Snap::Dict(d.entries.borrow().clone()));
                }
                (Value::Ref(r), _, Some(x)) => {
                    let x = self.decode(mk, x)?;
                    *r.value.borrow_mut() = x.clone();
                    self.snaps[i] = Some(Snap::Ref(x));
                    mk.ref_changed = true;
                }
                _ => return Err("a write that does not fit its node".to_string()),
            }
        }
        Ok(())
    }

    /// A value the host wrote, as a value of the VM.
    fn decode(&self, mk: &MakeValues, j: &Json) -> Result<Value, String> {
        Ok(match j {
            Json::Null => Value::Null,
            Json::Bool(b) => {
                if *b {
                    mk.booleans.0.clone()
                } else {
                    mk.booleans.1.clone()
                }
            }
            Json::Num(x) => num(*x),
            Json::Str(s) => string_value(s),
            Json::Arr(_) => return Err("an array where a value goes".to_string()),
            Json::Obj(o) => match o.first() {
                Some((k, Json::Num(i))) if k == "@" => self.nodes.get(*i as usize).cloned().ok_or("a node that is not there")?,
                Some((k, Json::Num(h))) if k == "h" => self.handles.get(*h as usize).cloned().ok_or("a handle that is not there")?,
                Some((k, Json::Num(o))) if k == "o" => host_obj(*o as i32),
                Some((k, Json::Num(i))) if k == "k" => KEPT.with(|t| t.borrow().0.get(&(*i as i64)).cloned()).ok_or("a kept value that is not there")?,
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
                        tail = Value::LList(Rc::new(LList { head: self.decode(mk, x)?, tail }));
                    }
                    tail
                }
                _ => return Err("an object that is not a value".to_string()),
            },
        })
    }
}

/// What making values of the VM needs of a Machine, so that a session can make them without one (a call
/// back has only the context of the call).
struct MakeValues {
    booleans: (Value, Value),
    dicts: usize,
    refs: usize,
    ref_changed: bool,
}

impl MakeValues {
    fn of(m: &Machine) -> MakeValues {
        MakeValues { booleans: (m.true_closure(), m.false_closure()), dicts: m.serials.dicts, refs: m.serials.refs, ref_changed: false }
    }
    fn back(self, m: &mut Machine) {
        m.serials.dicts = self.dicts;
        m.serials.refs = self.refs;
        if self.ref_changed {
            m.ref_changed = true;
        }
    }
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
/// "ok", "throw", "escape" or "yield" (the header of this file); after "yield", value is the LList
/// (generator request), and __JS_fn_resume__ goes on with the generator.
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
    let session = Rc::new(RefCell::new(Session { booleans: Some((m.true_closure(), m.false_closure())), ..Default::default() }));
    let out = session.borrow_mut().message(|s, out, made| {
        out.push_str("\"a\":[");
        let mut cur = args;
        let mut first = true;
        while let Value::LList(c) = cur {
            if !first {
                out.push(',');
            }
            first = false;
            s.value(&c.head, out, made);
            cur = c.tail.clone();
        }
        out.push(']');
    });
    let answer = in_session(m, vp, &session, || sys::jsfn_call(id as i32, &name, &out));
    finish(m, vp, prim, on_error, &session, answer)
}

/// __JS_fn_resume__ onError generator how value ,(status value): go on with the generator that a call
/// of __JS_fn_call__ (or of this) left at a yield: how is "next" (value is what the yield gives) or
/// "throw" (the yield throws value). The answer is that of __JS_fn_call__.
pub fn resume(m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__JS_fn_resume__";
    let mut a = take_n(vp, prim, 4)?;
    let value = a.pop().unwrap();
    let how = a.pop().unwrap();
    let generator = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Value::Number(g) = generator else {
        vp.prim_error(on_error, &format!("{}: expected a Number generator, got {}", prim, value_type_name(&generator)));
        return Ok(());
    };
    let how = match string_cow(&how) {
        Some(Ok(s)) if s == "next" || s == "throw" => s.into_owned(),
        _ => {
            vp.prim_error(on_error, &format!("{}: expected \"next\" or \"throw\"", prim));
            return Ok(());
        }
    };
    let Some(session) = GENERATORS.with(|gs| gs.borrow_mut().remove(&(g as i64))) else {
        vp.prim_error(on_error, &format!("{}: no generator {} waits", prim, g));
        return Ok(());
    };
    let out = session.borrow_mut().message(|s, out, made| {
        out.push_str(&format!("\"how\":\"{}\",\"v\":", how));
        s.value(&value, out, made);
    });
    let answer = in_session(m, vp, &session, || sys::jsfn_resume(g as i32, &out));
    finish(m, vp, prim, on_error, &session, answer)
}

/// Run f, a call of the host, with session as the innermost one, so that the host can call back.
fn in_session<T>(m: &Machine, vp: &mut VProc, session: &Rc<RefCell<Session>>, f: impl FnOnce() -> T) -> T {
    SESSIONS.with(|s| s.borrow_mut().push(session.clone()));
    let r = super::host::with_callback_context(m, vp, f);
    SESSIONS.with(|s| s.borrow_mut().pop());
    r
}

/// The answer of the host to a call or a resumption.
fn finish(m: &mut Machine, vp: &mut VProc, prim: &str, on_error: Value, session: &Rc<RefCell<Session>>, answer: Result<String, String>) -> PResult {
    let answer = match answer.and_then(|a| Json::parse(&a).map_err(|e| format!("the host gave {}", e))) {
        Ok(a) => a,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: {}", prim, e));
            return Ok(());
        }
    };
    let status = match answer.get("s") {
        Some(Json::Str(s)) => s.clone(),
        _ => {
            vp.prim_error(on_error, &format!("{}: the host gave no status", prim));
            return Ok(());
        }
    };
    if status == "error" || status == "escape" {
        let text = match answer.get("v") {
            Some(Json::Str(s)) => s.clone(),
            _ => String::new(),
        };
        if status == "error" {
            vp.prim_error(on_error, &format!("{}: {}", prim, text));
        } else {
            vp.prim_return2(string_value(&status), string_value(&text));
        }
        return Ok(());
    }
    let mut mk = MakeValues::of(m);
    let result = {
        let mut s = session.borrow_mut();
        s.take(&mut mk, &answer).and_then(|()| s.decode(&mk, answer.get("v").unwrap_or(&Json::Null)))
    };
    mk.back(m);
    let value = match result {
        Ok(v) => v,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: the host gave {}", prim, e));
            return Ok(());
        }
    };
    if status == "yield" {
        let Some(Json::Num(g)) = answer.get("g") else {
            vp.prim_error(on_error, &format!("{}: the host gave a yield without a generator", prim));
            return Ok(());
        };
        GENERATORS.with(|gs| gs.borrow_mut().insert(*g as i64, session.clone()));
        let pair = Value::LList(Rc::new(LList { head: num(*g), tail: Value::LList(Rc::new(LList { head: value, tail: Value::Null })) }));
        vp.prim_return2(string_value(&status), pair);
    } else {
        vp.prim_return2(string_value(&status), value);
    }
    Ok(())
}

/// A call back from the host during a call or a resumption (vm/wasm/src/lib.rs, conma_jsfn_callback):
/// the JSON of the host ({"f":value,"a":[...],"w":[...],"n":[...]}) and the JSON of the answer.
#[allow(dead_code)]
pub fn callback(request: &str) -> String {
    let quote = |s: &str| {
        let mut t = String::new();
        json_string(s, &mut t);
        t
    };
    let Some(session) = SESSIONS.with(|s| s.borrow().last().cloned()) else {
        return format!("{{\"e\":{}}}", quote("a call back while no function of JavaScript runs"));
    };
    let json = match Json::parse(request) {
        Ok(j) => j,
        Err(e) => return format!("{{\"e\":{}}}", quote(&e)),
    };
    let Some(booleans) = session.borrow().booleans.clone() else {
        return format!("{{\"e\":{}}}", quote("a session without Booleans"));
    };
    let mut mk = MakeValues { booleans, dicts: 0, refs: 0, ref_changed: false };
    // the host keeps a value of the call across calls: what it changed, then the value
    if let Some(x) = json.get("keep") {
        let kept = (|| {
            let mut s = session.borrow_mut();
            s.take(&mut mk, &json)?;
            s.decode(&mk, x)
        })();
        return match kept {
            Ok(v) => {
                let i = KEPT.with(|k| {
                    let mut k = k.borrow_mut();
                    k.1 += 1;
                    let i = k.1;
                    k.0.insert(i, v);
                    i
                });
                session.borrow_mut().message(|_, out, _| out.push_str(&format!("\"r\":{}", i)))
            }
            Err(e) => format!("{{\"e\":{}}}", quote(&e)),
        };
    }
    // what the host changed, then the function and its arguments
    let taken = (|| {
        let mut s = session.borrow_mut();
        s.take(&mut mk, &json)?;
        let f = s.decode(&mk, json.get("f").ok_or("no function")?)?;
        let mut args = Vec::new();
        if let Some(Json::Arr(a)) = json.get("a") {
            for x in a {
                args.push(s.decode(&mk, x)?);
            }
        }
        Ok::<_, String>((f, args))
    })();
    let (f, args) = match taken {
        Ok(x) => x,
        Err(e) => return format!("{{\"e\":{}}}", quote(&e)),
    };
    match super::host::run_nested(f, |_| Ok(args)) {
        Ok(results) => {
            let value = match results.len() {
                1 => results[0].clone(),
                _ => results.iter().rev().fold(Value::Null, |tail, head| Value::LList(Rc::new(LList { head: head.clone(), tail }))),
            };
            session.borrow_mut().message(|s, out, made| {
                out.push_str("\"r\":");
                s.value(&value, out, made);
            })
        }
        Err(super::host::CallbackError::Error(e)) => format!("{{\"e\":{}}}", quote(&e)),
        Err(super::host::CallbackError::Escape(e)) => format!("{{\"x\":{}}}", quote(&e)),
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
