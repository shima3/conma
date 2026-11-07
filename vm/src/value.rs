//! ConMa values. They mirror the Python classes of test/vproc_step.py, so that
//! traces and error messages come out the same.

use crate::module::{BodyDef, GVEnv};
use crate::prim::PrimDef;
use crate::sexp::Sexp;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The SInfo of a Body: the atoms after `(SInfo pos`, such as
/// `"file.se" "12" "3"`.
pub type SInfo = Rc<Vec<Sexp>>;

#[derive(Clone)]
pub enum Value {
    Null,
    /// A String made at run time: its text, not escaped.
    Str(Rc<str>),
    /// A String literal of the program: its AST node `(String pos "text")`.
    StrNode(Sexp),
    /// Any other AST node that ended up as a value.
    Raw(Sexp),
    /// A Number. It has no identity: two Numbers are the same value when they
    /// are equal, NaN being the same as NaN (`same`).
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
    /// An OS file descriptor.
    Stream(Rc<StreamObj>),
    /// An OS process started by __OS_spawn__.
    OSProc(Rc<OSProcObj>),
}

/// A raw OS file descriptor as a ConMa value.
pub struct StreamObj {
    pub serial: usize,
    pub fd: i32,
    /// 'r' or 'w'
    pub mode: char,
    pub closed: std::cell::Cell<bool>,
    /// One of the interpreter's own fds 0, 1, 2, which stay open.
    pub std: bool,
    /// Bytes of a UTF-8 character split between reads.
    pub pending: RefCell<Vec<u8>>,
}

impl StreamObj {
    pub fn new(serial: usize, fd: i32, mode: char, std: bool) -> StreamObj {
        StreamObj { serial, fd, mode, closed: std::cell::Cell::new(false), std, pending: RefCell::new(Vec::new()) }
    }
    pub fn kind(&self) -> &'static str {
        if self.mode == 'r' {
            "input Stream"
        } else {
            "output Stream"
        }
    }
    /// Close the fd (fds 0, 1, 2 stay open). Marked closed first: the fd
    /// number may be reused by a later pipe.
    pub fn close(&self) -> std::io::Result<()> {
        if !self.closed.get() {
            self.closed.set(true);
            if !self.std {
                return crate::sys::close_fd(self.fd);
            }
        }
        Ok(())
    }
    /// Decode bytes as UTF-8 across reads, replacing invalid bytes with U+FFFD.
    pub fn decode(&self, data: &[u8], last: bool) -> String {
        let mut buf = self.pending.borrow_mut();
        buf.extend_from_slice(data);
        let mut out = String::new();
        let mut i = 0;
        loop {
            match std::str::from_utf8(&buf[i..]) {
                Ok(s) => {
                    out.push_str(s);
                    i = buf.len();
                    break;
                }
                Err(e) => {
                    let ok = e.valid_up_to();
                    out.push_str(std::str::from_utf8(&buf[i..i + ok]).unwrap());
                    i += ok;
                    match e.error_len() {
                        Some(n) => {
                            out.push('\u{FFFD}');
                            i += n;
                        }
                        None => {
                            if last {
                                out.push('\u{FFFD}');
                                i = buf.len();
                            }
                            break;
                        }
                    }
                }
            }
        }
        buf.drain(..i);
        out
    }
}

pub struct OSProcObj {
    pub serial: usize,
    pub child: RefCell<std::process::Child>,
    /// The exit code, once the process has been waited for.
    pub code: std::cell::Cell<Option<i32>>,
}

impl OSProcObj {
    /// The exit code (a negative signal number if a signal killed it), or
    /// None if the process is still running (Popen.poll).
    pub fn poll(&self) -> std::io::Result<Option<i32>> {
        if let Some(c) = self.code.get() {
            return Ok(Some(c));
        }
        use std::os::unix::process::ExitStatusExt;
        let st = self.child.borrow_mut().try_wait()?;
        let code = st.map(|s| s.code().unwrap_or_else(|| -s.signal().unwrap_or(0)));
        self.code.set(code);
        Ok(code)
    }
}

/// A Number.
pub fn num(x: f64) -> Value {
    Value::Number(x)
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
    pub index: RefCell<DictIndex>,
}

/// A fast hash for the index of a Dict (Fx, as in rustc): Dicts do not need
/// the protection of SipHash against keys made to collide.
#[derive(Default, Clone, Copy)]
pub struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.add(u64::from_le_bytes(c.try_into().unwrap()));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut b = [0u8; 8];
            b[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(b) ^ ((rest.len() as u64) << 56));
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.add(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.add(i);
    }
    fn write_usize(&mut self, i: usize) {
        self.add(i as u64);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

impl FxHasher {
    #[inline]
    fn add(&mut self, w: u64) {
        self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
}

pub type FxBuild = std::hash::BuildHasherDefault<FxHasher>;

/// The hash of a String key in the index of a Dict: Fx over its bytes, with
/// the high bits mixed into the low ones. A const fn, so that transpiled code
/// has the hash of a key written in place as a constant.
pub const fn str_hash(s: &str) -> u64 {
    const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
    let b = s.as_bytes();
    let mut h: u64 = 0;
    let mut i = 0;
    while i + 8 <= b.len() {
        let w = u64::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3], b[i + 4], b[i + 5], b[i + 6], b[i + 7]]);
        h = (h.rotate_left(5) ^ w).wrapping_mul(K);
        i += 8;
    }
    if i < b.len() {
        let mut w: u64 = 0;
        let mut k = 0;
        while i + k < b.len() {
            w |= (b[i + k] as u64) << (8 * k);
            k += 1;
        }
        h = (h.rotate_left(5) ^ (w ^ ((k as u64) << 56))).wrapping_mul(K);
    }
    h ^ (h >> 29)
}

/// A String key in the index, with its str_hash.
#[derive(Clone)]
struct StrKey {
    hash: u64,
    text: Box<str>,
}

/// A String key as the index looks it up: a StrKey, or a text with its hash.
trait StrLike {
    fn key_hash(&self) -> u64;
    fn key_text(&self) -> &str;
}

impl StrLike for StrKey {
    fn key_hash(&self) -> u64 {
        self.hash
    }
    fn key_text(&self) -> &str {
        &self.text
    }
}

impl StrLike for (u64, &str) {
    fn key_hash(&self) -> u64 {
        self.0
    }
    fn key_text(&self) -> &str {
        self.1
    }
}

impl<'a> std::borrow::Borrow<dyn StrLike + 'a> for StrKey {
    fn borrow(&self) -> &(dyn StrLike + 'a) {
        self
    }
}

impl std::hash::Hash for dyn StrLike + '_ {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        h.write_u64(self.key_hash());
    }
}

impl PartialEq for dyn StrLike + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.key_hash() == other.key_hash() && self.key_text() == other.key_text()
    }
}

impl Eq for dyn StrLike + '_ {}

impl std::hash::Hash for StrKey {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        h.write_u64(self.hash);
    }
}

impl PartialEq for StrKey {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.text == other.text
    }
}

impl Eq for StrKey {}

/// The hasher of the String index: the key is hashed already (str_hash).
#[derive(Default, Clone, Copy)]
pub struct HashedHasher(u64);

impl std::hash::Hasher for HashedHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("String keys are hashed with str_hash")
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

/// The index of a Dict: String keys by their text (looked up without making a
/// String), other keys by their DictKey.
#[derive(Default, Clone)]
pub struct DictIndex {
    strs: HashMap<StrKey, usize, std::hash::BuildHasherDefault<HashedHasher>>,
    others: HashMap<DictKey, usize, FxBuild>,
}

/// How a key is found in the index: a String by its text, another value by
/// its DictKey, NaN not at all (it is not equal even to itself).
pub enum IndexKey<'a> {
    Str(std::borrow::Cow<'a, str>),
    Other(DictKey),
    None,
}

pub fn index_key(v: &Value) -> Result<IndexKey<'_>, String> {
    use std::borrow::Cow;
    match v {
        Value::Str(s) => return Ok(IndexKey::Str(Cow::Borrowed(s))),
        Value::StrNode(n) => {
            let text = n.as_list().and_then(|l| l.get(2)).and_then(|x| x.as_atom()).map(unquote).unwrap_or("");
            return Ok(IndexKey::Str(if text.contains('\\') { Cow::Owned(unescape(text)?) } else { Cow::Borrowed(text) }));
        }
        _ => {}
    }
    Ok(match dict_key(v)? {
        Some(k) => IndexKey::Other(k),
        None => IndexKey::None,
    })
}

impl DictObj {
    /// The position of the String key with this text (a key that is not a
    /// String is never equal to it).
    #[inline]
    pub fn find_str(&self, text: &str) -> Option<usize> {
        self.find_str_hashed(text, str_hash(text))
    }

    /// find_str with the str_hash of the text.
    #[inline]
    pub fn find_str_hashed(&self, text: &str, hash: u64) -> Option<usize> {
        self.index.borrow().strs.get(&(hash, text) as &dyn StrLike).copied()
    }
}

impl DictIndex {
    /// The position of the key, or None; Err(()) for a key that is never in
    /// the index (NaN).
    pub fn get(&self, k: &IndexKey) -> Result<Option<usize>, ()> {
        match k {
            IndexKey::Str(s) => Ok(self.strs.get(&(str_hash(s), &**s) as &dyn StrLike).copied()),
            IndexKey::Other(d) => Ok(self.others.get(d).copied()),
            IndexKey::None => Err(()),
        }
    }
    pub fn insert(&mut self, k: IndexKey, i: usize) {
        match k {
            IndexKey::Str(s) => {
                self.strs.insert(StrKey { hash: str_hash(&s), text: s.into_owned().into_boxed_str() }, i);
            }
            IndexKey::Other(d) => {
                self.others.insert(d, i);
            }
            IndexKey::None => {}
        }
    }
    pub fn clear(&mut self) {
        self.strs.clear();
        self.others.clear();
    }
}

/// The CChain of the caller of the function that called __Cont_get__.
pub struct ContObj {
    pub serial: usize,
    /// Changed only by native code (vm/src/native.rs): a continuation made
    /// while native code runs a call as a Rust call has a CChain without the
    /// CFrames of the pending Rust calls, and gets them when they end.
    pub cchain: RefCell<Value>,
    /// The Rust call of native code that made it (native::Frame), or 0.
    pub owner: Cell<u64>,
}

impl ContObj {
    pub fn new(serial: usize, cchain: Value) -> ContObj {
        ContObj { serial, cchain: RefCell::new(cchain), owner: Cell::new(0) }
    }
}

/// A key that is equal for two values exactly when values_equal holds.
#[derive(Hash, PartialEq, Eq, Clone)]
pub enum DictKey {
    Null,
    Num(u64),
    Str(String),
    Obj(usize),
}

/// The identity of a value that is equal only to itself (not used for
/// Numbers, which have none).
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
        Value::Stream(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::OSProc(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Prim(p) => *p as *const _ as *const u8 as usize,
        Value::Raw(s) | Value::StrNode(s) => match s {
            Sexp::List(l) => Rc::as_ptr(l) as *const u8 as usize,
            Sexp::Atom(a) => Rc::as_ptr(a) as *const u8 as usize,
        },
        Value::Str(s) => Rc::as_ptr(s) as *const u8 as usize,
        Value::Number(x) => x.to_bits() as usize,
        Value::Null => 0,
    }
}

/// Whether a and b are the same value (__Ref_wait_change__, __nop__):
/// Numbers when they are equal, any other value only itself.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        // +0 and -0 are the same, and NaN is the same as NaN.
        (Value::Number(x), Value::Number(y)) => x == y || (x.is_nan() && y.is_nan()),
        (Value::Number(..), _) | (_, Value::Number(..)) => false,
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
        if let Ok(found) = self.index.borrow().get(&index_key(key)?) {
            return Ok(found);
        }
        for (i, (k, _)) in self.entries.borrow().iter().enumerate() {
            if values_equal(k, key)? {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    pub fn append(&self, key: Value, value: Value) -> Result<(), String> {
        let n = self.entries.borrow().len();
        self.index.borrow_mut().insert(index_key(&key)?, n);
        self.entries.borrow_mut().push((key, value));
        Ok(())
    }
    pub fn remove(&self, i: usize) -> Result<(), String> {
        self.entries.borrow_mut().remove(i);
        let mut index = DictIndex::default();
        for (i, (k, _)) in self.entries.borrow().iter().enumerate() {
            index.insert(index_key(k)?, i);
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

/// A String value of text made at run time. It keeps the text itself; the
/// source form is made only for traces (string_source).
pub fn string_value(text: &str) -> Value {
    Value::Str(Rc::from(text))
}

/// The source form of a String made at run time: escaped and quoted.
pub fn string_source(text: &str) -> String {
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
    out
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
        Value::Str(s) => Some(Ok(s.to_string())),
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
        Value::Number(..) => "Number".into(),
        Value::LList(_) => "LList".into(),
        Value::VProc(_) => "VProc".into(),
        Value::Stream(s) => s.kind().into(),
        Value::OSProc(_) => "OSProc".into(),
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
        Value::Number(..) => "Number",
        Value::Ref(_) => "Ref",
        Value::MSeq(_) => "MSeq",
        Value::Dict(_) => "Dict",
        Value::CFrame(_) => "CFrame",
        Value::LList(_) => "LList",
        Value::VProc(_) => "VProc",
        Value::Stream(_) => "Stream",
        Value::OSProc(_) => "OSProc",
        Value::Raw(_) | Value::Null => return Err("__type_name__: unknown value".into()),
    })
}
