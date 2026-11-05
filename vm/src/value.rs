//! ConMa values. They mirror the Python classes of test/vproc_step.py, so that
//! traces and error messages come out the same.

use crate::module::{BodyDef, GVEnv};
use crate::prim::PrimDef;
use crate::sexp::Sexp;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The SInfo of a Body: the atoms after `(SInfo pos`, such as
/// `"file.se" "12" "3"`.
pub type SInfo = Rc<Vec<Sexp>>;

#[derive(Clone)]
pub enum Value {
    Null,
    /// A String made at run time, in source form: quoted and escaped.
    Str(Rc<str>),
    /// A String literal of the program: its AST node `(String pos "text")`.
    StrNode(Sexp),
    /// Any other AST node that ended up as a value.
    Raw(Sexp),
    Number(f64),
    Closure(Rc<Closure>),
    Seq(Rc<SeqClosure>),
    Prim(&'static PrimDef),
    CFrame(Rc<CFrame>),
    LList(Rc<LList>),
    Ref(Rc<RefObj>),
    MSeq(Rc<MSeqObj>),
    Dict(Rc<DictObj>),
    /// The escape continuation that __Cont_get__ passes.
    Cont(Rc<ContObj>),
    /// A VProc, as __VProc_current__ and __VProc_spawn__ return it.
    VProc(Rc<crate::vproc::VProcCell>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        match self {
            Value::Null => true,
            Value::Raw(s) | Value::StrNode(s) => s.tag() == Some("Null"),
            _ => false,
        }
    }
}

/// A mutable cell. Traces show it by its serial number, because it may contain itself.
pub struct RefObj {
    pub serial: usize,
    pub value: RefCell<Value>,
    /// For a LazyList that a VProc evaluates: lazy_chain and lazy_thunk.
    pub lazy: RefCell<Option<LazyInfo>>,
}

impl RefObj {
    pub fn new(serial: usize, value: Value) -> RefObj {
        RefObj { serial, value: RefCell::new(value), lazy: RefCell::new(None) }
    }
}

/// lazy_chain and lazy_thunk of a LazyList that a VProc evaluates: the CChain
/// of the VProc when it claimed the LazyList (None for HELD, made by
/// __LazyList_hold__), and the thunk.
#[derive(Clone)]
pub struct LazyInfo {
    pub chain: Option<Value>,
    pub thunk: Value,
}

pub struct MSeqObj {
    pub serial: usize,
    pub items: RefCell<Vec<Value>>,
}

/// A mapping in insertion order. Keys are compared with values_equal; the
/// index finds a key that has a dict_key without comparing it with every key.
pub struct DictObj {
    pub serial: usize,
    pub entries: RefCell<Vec<(Value, Value)>>,
    pub index: RefCell<HashMap<DictKey, usize>>,
}

/// The CChain of the caller of the function that called __Cont_get__.
pub struct ContObj {
    pub serial: usize,
    pub cchain: Value,
}

/// A key that is equal for two values exactly when values_equal holds.
#[derive(Hash, PartialEq, Eq, Clone)]
pub enum DictKey {
    Null,
    Num(u64),
    Str(String),
    Obj(usize),
}

/// The identity of a value that is equal only to itself.
pub fn identity(v: &Value) -> usize {
    match v {
        Value::Closure(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Seq(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::CFrame(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::LList(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Ref(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::MSeq(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Dict(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Cont(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::VProc(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Prim(p) => *p as *const _ as *const u8 as usize,
        Value::Raw(s) | Value::StrNode(s) => match s {
            Sexp::List(l) => Rc::as_ptr(l) as *const u8 as usize,
            Sexp::Atom(a) => Rc::as_ptr(a) as *const u8 as usize,
        },
        Value::Str(s) => Rc::as_ptr(s) as *const u8 as usize,
        Value::Number(_) | Value::Null => 0,
    }
}

pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Number(_), _) | (_, Value::Number(_)) => false,
        (Value::Null, _) | (_, Value::Null) => false,
        _ => identity(a) == identity(b),
    }
}

/// dict_key: None for a value that is not equal even to itself (NaN).
pub fn dict_key(v: &Value) -> Result<Option<DictKey>, String> {
    if let Value::Number(x) = v {
        if x.is_nan() {
            return Ok(None);
        }
        let x = if *x == 0.0 { 0.0f64 } else { *x };
        return Ok(Some(DictKey::Num(x.to_bits())));
    }
    if v.is_null() {
        return Ok(Some(DictKey::Null));
    }
    if let Some(t) = string_text(v) {
        return Ok(Some(DictKey::Str(t?)));
    }
    Ok(Some(DictKey::Obj(identity(v))))
}

/// Key equality for PDict and Dict: Strings by content, Numbers by value,
/// null equals null, and any other value only itself.
pub fn values_equal(a: &Value, b: &Value) -> Result<bool, String> {
    let (ta, tb) = (value_type_name(a), value_type_name(b));
    if ta != tb {
        return Ok(false);
    }
    if ta == "null" {
        return Ok(true);
    }
    if ta == "String" {
        if let (Some(x), Some(y)) = (string_text(a), string_text(b)) {
            return Ok(x? == y?);
        }
    }
    if let (Value::Number(x), Value::Number(y)) = (a, b) {
        return Ok(x == y);
    }
    Ok(same(a, b))
}

impl DictObj {
    pub fn find(&self, key: &Value) -> Result<Option<usize>, String> {
        if let Some(k) = dict_key(key)? {
            return Ok(self.index.borrow().get(&k).copied());
        }
        for (i, (k, _)) in self.entries.borrow().iter().enumerate() {
            if values_equal(k, key)? {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn append(&self, key: Value, value: Value) -> Result<(), String> {
        if let Some(k) = dict_key(&key)? {
            self.index.borrow_mut().insert(k, self.entries.borrow().len());
        }
        self.entries.borrow_mut().push((key, value));
        Ok(())
    }
    pub fn remove(&self, i: usize) -> Result<(), String> {
        self.entries.borrow_mut().remove(i);
        let mut index = HashMap::new();
        for (i, (k, _)) in self.entries.borrow().iter().enumerate() {
            if let Some(dk) = dict_key(k)? {
                index.insert(dk, i);
            }
        }
        *self.index.borrow_mut() = index;
        Ok(())
    }
}

pub struct Closure {
    /// All parameter names of the Function; the first `skip` are already bound.
    pub params: Rc<[Rc<str>]>,
    pub skip: usize,
    pub body: Option<Rc<BodyDef>>,
    pub lvenv: LVEnv,
    /// GVEnv of the module the Closure was defined in.
    pub gvenv: Option<Rc<GVEnv>>,
    pub variadic: bool,
}

impl Closure {
    pub fn params(&self) -> &[Rc<str>] {
        &self.params[self.skip..]
    }
}

/// A Seq Closure `,(__Sink__) __Sink__ v1 v2 ...`.
pub struct SeqClosure {
    pub olist: Vec<Value>,
    pub sinfo: Option<SInfo>,
    pub lvenv: LVEnv,
}

pub struct CFrame {
    pub closure: Value,
    pub sinfo: Option<SInfo>,
}

pub struct LList {
    pub head: Value,
    /// An LList or null.
    pub tail: Value,
}

impl Drop for LList {
    // Drop a long list without recursion.
    fn drop(&mut self) {
        let mut next = std::mem::replace(&mut self.tail, Value::Null);
        while let Value::LList(rc) = next {
            match Rc::try_unwrap(rc) {
                Ok(mut cell) => next = std::mem::replace(&mut cell.tail, Value::Null),
                Err(_) => break,
            }
        }
    }
}

/// A local environment: the innermost binding first, as the Python list
/// `[(name, value), ...]`, but shared between Closures.
pub type LVEnv = Option<Rc<LVNode>>;

pub struct LVNode {
    pub name: Rc<str>,
    pub value: Value,
    pub next: LVEnv,
}

impl Drop for LVNode {
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(rc) = next {
            match Rc::try_unwrap(rc) {
                Ok(mut node) => next = node.next.take(),
                Err(_) => break,
            }
        }
    }
}

pub fn lv_push(env: &LVEnv, name: Rc<str>, value: Value) -> LVEnv {
    Some(Rc::new(LVNode { name, value, next: env.clone() }))
}

pub fn lv_get(env: &LVEnv, idx: usize) -> Option<&Value> {
    let mut cur = env.as_ref();
    let mut i = idx;
    while let Some(node) = cur {
        if i == 0 {
            return Some(&node.value);
        }
        i -= 1;
        cur = node.next.as_ref();
    }
    None
}

pub fn lv_iter(env: &LVEnv) -> impl Iterator<Item = &LVNode> {
    let mut cur = env.as_deref();
    std::iter::from_fn(move || {
        let node = cur?;
        cur = node.next.as_deref();
        Some(node)
    })
}

// ── Strings ──────────────────────────────────────────────────────────────────

pub fn unquote(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

pub fn quoted(s: &str) -> String {
    if s.starts_with('"') && s.ends_with('"') {
        s.to_string()
    } else {
        format!("\"{}\"", s)
    }
}

/// A String value of text made at run time: escaped and quoted.
pub fn string_value(text: &str) -> Value {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    Value::Str(Rc::from(out))
}

/// Process the escapes \" \\ \n \t \r of the text of a String.
pub fn unescape(s: &str) -> Result<String, String> {
    if !s.contains('\\') {
        return Ok(s.to_string());
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(ch) = it.next() {
        if ch == '\\' {
            match it.next() {
                None => return Err(format!("unescape: trailing backslash in {}", py_repr(s))),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some(c) => {
                    return Err(format!("unescape: unknown escape \\{} in {}", c, py_repr(s)))
                }
            }
        } else {
            out.push(ch);
        }
    }
    Ok(out)
}

/// The text of a String value (StrNode or Str), unescaped.
pub fn string_text(v: &Value) -> Option<Result<String, String>> {
    match v {
        Value::Str(s) => Some(unescape(unquote(s))),
        Value::StrNode(n) => {
            let l = n.as_list()?;
            Some(match l.get(2).and_then(|x| x.as_atom()) {
                Some(a) => unescape(unquote(a)),
                None => Ok(String::new()),
            })
        }
        _ => None,
    }
}

/// Python's repr() of a str, for error messages.
pub fn py_repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(q);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if c == q => {
                out.push('\\');
                out.push(c)
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(q);
    out
}

// ── Numbers ──────────────────────────────────────────────────────────────────

/// Integer values without a decimal point (below 1e21), other finite values
/// as Python's repr() writes them, and inf, -inf, nan.
pub fn number_to_str(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if x == x.trunc() && x.abs() < 1e21 {
        return format!("{}", x as i128);
    }
    py_float_repr(x)
}

/// Python's repr() of a finite float: the shortest digits that read back the
/// same, in fixed notation when the exponent is in -4..16, otherwise as
/// d.ddde+XX.
pub fn py_float_repr(x: f64) -> String {
    let e = format!("{:e}", x); // shortest round-trip digits, e.g. "-1.5e-5"
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let sign = if neg { "-" } else { "" };
    if (-4..16).contains(&exp) {
        let n = digits.len() as i32;
        let s = if exp >= 0 {
            let int_len = exp + 1;
            if n <= int_len {
                format!("{}{}.0", digits, "0".repeat((int_len - n) as usize))
            } else {
                format!("{}.{}", &digits[..int_len as usize], &digits[int_len as usize..])
            }
        } else {
            format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        };
        format!("{}{}", sign, s)
    } else {
        let m = if digits.len() > 1 {
            format!("{}.{}", &digits[..1], &digits[1..])
        } else {
            digits.clone()
        };
        let es = if exp < 0 { format!("-{:02}", -exp) } else { format!("+{:02}", exp) };
        format!("{}{}e{}", sign, m, es)
    }
}

/// The type name used in error messages (value_type_name).
pub fn value_type_name(v: &Value) -> String {
    if v.is_null() {
        return "null".into();
    }
    match v {
        Value::Closure(_) | Value::Seq(_) | Value::Prim(_) | Value::Cont(_) => "Closure".into(),
        Value::CFrame(_) => "CFrame".into(),
        Value::Ref(_) => "Ref".into(),
        Value::MSeq(_) => "MSeq".into(),
        Value::Dict(_) => "Dict".into(),
        Value::StrNode(n) | Value::Raw(n) => match n.tag() {
            Some(t) => t.to_string(),
            None => "list".into(),
        },
        Value::Str(_) => "String".into(),
        Value::Number(_) => "Number".into(),
        Value::LList(_) => "LList".into(),
        Value::VProc(_) => "VProc".into(),
        Value::Null => "null".into(),
    }
}

/// The type name that __type_name__ gives.
pub fn type_name(v: &Value) -> Result<&'static str, String> {
    if v.is_null() {
        return Ok("null");
    }
    Ok(match v {
        Value::Str(_) | Value::StrNode(_) => "String",
        Value::Closure(_) | Value::Seq(_) | Value::Prim(_) | Value::Cont(_) => "Closure",
        Value::Number(_) => "Number",
        Value::Ref(_) => "Ref",
        Value::MSeq(_) => "MSeq",
        Value::Dict(_) => "Dict",
        Value::CFrame(_) => "CFrame",
        Value::LList(_) => "LList",
        Value::VProc(_) => "VProc",
        Value::Raw(_) | Value::Null => return Err("__type_name__: unknown value".into()),
    })
}
