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
    /// A WebAssembly module made by __Wasm_compile__.
    Wasm(Rc<WasmObj>),
    /// The values of the variables of a frame (module::frame_chain): only the value of an LVEnv
    /// node, never one that a program sees.
    Frame(Rc<Vec<Value>>),
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
    /// Close the fd when the last reference to the Stream is dropped (the
    /// outputStream of __JS_call__: the host then knows that the program no
    /// longer holds the values of that call; see 2.5 of consh/js/PLAN-ja.md).
    pub drop_close: std::cell::Cell<bool>,
}

impl Drop for StreamObj {
    fn drop(&mut self) {
        if self.drop_close.get() {
            let _ = self.close();
        }
    }
}

impl StreamObj {
    pub fn new(serial: usize, fd: i32, mode: char, std: bool) -> StreamObj {
        StreamObj {
            serial,
            fd,
            mode,
            closed: std::cell::Cell::new(false),
            std,
            pending: RefCell::new(Vec::new()),
            drop_close: std::cell::Cell::new(false),
        }
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

/// A WebAssembly module of the host (__Wasm_compile__), removed from the host
/// when its last reference is dropped.
pub struct WasmObj {
    pub serial: usize,
    /// The number of the module in the host.
    pub host: i32,
    /// The exported functions and their numbers of parameters.
    pub exports: Vec<(String, usize)>,
}

impl Drop for WasmObj {
    fn drop(&mut self) {
        crate::sys::wasm_free(self.host);
    }
}

pub struct OSProcObj {
    pub serial: usize,
    #[cfg(unix)]
    pub child: RefCell<std::process::Child>,
    /// The number of the process in the host (the VM for wasm32, consh/js).
    #[cfg(not(unix))]
    pub host: i32,
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
        #[cfg(unix)]
        let code = {
            use std::os::unix::process::ExitStatusExt;
            let st = self.child.borrow_mut().try_wait()?;
            st.map(|s| s.code().unwrap_or_else(|| -s.signal().unwrap_or(0)))
        };
        #[cfg(not(unix))]
        let code = crate::sys::proc_poll(self.host)?;
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

/// A mutable sequence. Its items are those of `items` from `start` on: the
/// rest that __MSeq_uncons__ returns shares the items of the MSeq it was
/// taken from (one more start) instead of copying them, and an MSeq copies its
/// items (from its start) before it changes them when another MSeq shares them
/// (items_mut), so that each behaves as an MSeq of its own.
pub struct MSeqObj {
    pub serial: usize,
    pub items: RefCell<Rc<Vec<Value>>>,
    pub start: Cell<usize>,
}

impl MSeqObj {
    pub fn new(serial: usize, items: Vec<Value>) -> MSeqObj {
        MSeqObj { serial, items: RefCell::new(Rc::new(items)), start: Cell::new(0) }
    }
    /// The items, from start on.
    pub fn to_vec(&self) -> Vec<Value> {
        self.items.borrow()[self.start.get()..].to_vec()
    }
    pub fn is_empty(&self) -> bool {
        self.items.borrow().len() <= self.start.get()
    }
    /// The first item, if any.
    pub fn head(&self) -> Option<Value> {
        self.items.borrow().get(self.start.get()).cloned()
    }
    /// The MSeq of the items after the first, sharing them.
    pub fn rest(&self, serial: usize) -> MSeqObj {
        MSeqObj { serial, items: RefCell::new(self.items.borrow().clone()), start: Cell::new(self.start.get() + 1) }
    }
    /// The items to change, from index 0: copied first (from start on) if
    /// another MSeq shares them or they do not start at 0.
    pub fn items_mut(&self) -> std::cell::RefMut<'_, Vec<Value>> {
        let mut items = self.items.borrow_mut();
        let start = self.start.get();
        if start > 0 && Rc::strong_count(&items) > 1 {
            *items = Rc::new(items[start..].to_vec());
            self.start.set(0);
        } else if start > 0 {
            Rc::make_mut(&mut items).drain(..start);
            self.start.set(0);
        }
        std::cell::RefMut::map(items, Rc::make_mut)
    }
}

/// A mapping in insertion order. Keys are compared with values_equal; the
/// index finds a key that has a dict_key without comparing it with every key.
/// A copy (__Dict_copy__) shares the entries and the index of the Dict it
/// was copied from until one of them is changed (entries_mut, index_mut).
pub struct DictObj {
    pub serial: usize,
    pub entries: RefCell<Rc<Vec<(Value, Value)>>>,
    pub index: RefCell<Rc<DictIndex>>,
    /// The shape of its keys (SHAPE_EMPTY for a new Dict, SHAPE_NONE for one
    /// that has no shape): see shape_after.
    pub shape: Cell<u32>,
}

// ── Shapes ──
// A Dict whose keys are all Strings, put in one after another and never
// removed, has the shape of that sequence of keys: Dicts with the same shape
// have each key at the same position. Transpiled code keeps, at each place
// where it reads or writes a Dict with a String key written in place, the
// last shape and position it found there (Ic), and finds the key without the
// index when the shape is the same. Shapes do not change what a program does.

/// No shape: a key was removed, a key is not a String, or there are too many.
pub const SHAPE_NONE: u32 = 0;
/// The shape of a Dict without keys.
pub const SHAPE_EMPTY: u32 = 1;
/// A Dict with more keys than this has no shape (a Dict used as a table).
const SHAPE_MAX_KEYS: u32 = 64;
/// The shapes made at most (a program that makes many tables of keys).
const SHAPE_MAX: usize = 1 << 16;

/// A shape: its number of keys, and the shape after putting in each next key
/// (by the str_hash and the text of the key).
struct ShapeNode {
    keys: u32,
    next: HashMap<u64, Vec<(Box<str>, u32)>, FxBuild>,
}

thread_local! {
    static SHAPES: RefCell<Vec<ShapeNode>> = RefCell::new(vec![
        ShapeNode { keys: 0, next: HashMap::default() },
        ShapeNode { keys: 0, next: HashMap::default() },
    ]);
}

/// The shape of a Dict of shape `shape` after a new String key `text` is put in.
pub fn shape_after(shape: u32, text: &str, hash: u64) -> u32 {
    if shape == SHAPE_NONE {
        return SHAPE_NONE;
    }
    SHAPES.with(|shapes| {
        let mut shapes = shapes.borrow_mut();
        let n = shapes.len();
        let node = &mut shapes[shape as usize];
        if let Some(v) = node.next.get(&hash) {
            if let Some((_, s)) = v.iter().find(|(t, _)| same_text(t, text)) {
                return *s;
            }
        }
        if node.keys >= SHAPE_MAX_KEYS || n >= SHAPE_MAX {
            return SHAPE_NONE;
        }
        let keys = node.keys + 1;
        node.next.entry(hash).or_default().push((text.into(), n as u32));
        shapes.push(ShapeNode { keys, next: HashMap::default() });
        n as u32
    })
}

/// The cache of a place in transpiled code that reads or writes a Dict with a
/// String key written in place: the last shape (high 32 bits) and the
/// position of the key in a Dict of that shape (low 32 bits).
pub struct Ic(Cell<u64>);

// The VM runs in one thread (its values are Rc).
unsafe impl Sync for Ic {}

impl Ic {
    pub const fn new() -> Ic {
        Ic(Cell::new(0))
    }
}

impl Default for Ic {
    fn default() -> Ic {
        Ic::new()
    }
}

impl DictObj {
    /// find_str_hashed through the cache of a place: the position of the key
    /// in a Dict of the cached shape, without the index.
    #[inline]
    pub fn find_str_cached(&self, text: &str, hash: u64, ic: &Ic) -> Option<usize> {
        let shape = self.shape.get();
        let c = ic.0.get();
        if shape != SHAPE_NONE && (c >> 32) as u32 == shape {
            return Some(c as u32 as usize);
        }
        let found = self.find_str_hashed(text, hash);
        if let (Some(i), true) = (found, shape != SHAPE_NONE) {
            ic.0.set(((shape as u64) << 32) | i as u64);
        }
        found
    }
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

/// The String keys of the index with one str_hash: almost always one.
#[derive(Clone)]
enum StrSlot {
    One(Box<str>, usize),
    Many(Vec<(Box<str>, usize)>),
}

impl StrSlot {
    #[inline]
    fn get(&self, text: &str) -> Option<usize> {
        match self {
            StrSlot::One(t, i) => same_text(t, text).then_some(*i),
            StrSlot::Many(v) => v.iter().find(|(t, _)| same_text(t, text)).map(|(_, i)| *i),
        }
    }
}

/// Whether two texts are the same: a short one byte by byte, without a call
/// of memcmp.
#[inline]
fn same_text(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    if a.len() > 16 {
        return a == b;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

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
    /// String keys by their str_hash (the hash map compares only the u64).
    strs: HashMap<u64, StrSlot, std::hash::BuildHasherDefault<HashedHasher>>,
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
        self.index.borrow().strs.get(&hash).and_then(|slot| slot.get(text))
    }
}

impl DictIndex {
    /// The position of the key, or None; Err(()) for a key that is never in
    /// the index (NaN).
    pub fn get(&self, k: &IndexKey) -> Result<Option<usize>, ()> {
        match k {
            IndexKey::Str(s) => Ok(self.strs.get(&str_hash(s)).and_then(|slot| slot.get(s))),
            IndexKey::Other(d) => Ok(self.others.get(d).copied()),
            IndexKey::None => Err(()),
        }
    }
    pub fn insert(&mut self, k: IndexKey, i: usize) {
        match k {
            IndexKey::Str(s) => {
                let text: Box<str> = s.into_owned().into_boxed_str();
                match self.strs.entry(str_hash(&text)) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        e.insert(StrSlot::One(text, i));
                    }
                    std::collections::hash_map::Entry::Occupied(mut e) => {
                        let slot = e.get_mut();
                        if let StrSlot::One(t, j) = slot {
                            if same_text(t, &text) {
                                *j = i;
                                return;
                            }
                            *slot = StrSlot::Many(vec![(t.clone(), *j)]);
                        }
                        if let StrSlot::Many(v) = slot {
                            match v.iter_mut().find(|(t, _)| same_text(t, &text)) {
                                Some((_, j)) => *j = i,
                                None => v.push((text, i)),
                            }
                        }
                    }
                }
            }
            IndexKey::Other(d) => {
                self.others.insert(d, i);
            }
            IndexKey::None => {}
        }
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
        Value::Wasm(r) => Rc::as_ptr(r) as *const u8 as usize,
        Value::Frame(r) => Rc::as_ptr(r) as *const u8 as usize,
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
    /// A new Dict without keys.
    pub fn new(serial: usize) -> DictObj {
        DictObj { serial, entries: RefCell::new(Rc::new(Vec::new())), index: RefCell::new(Rc::new(DictIndex::default())), shape: Cell::new(SHAPE_EMPTY) }
    }
    /// A copy that shares the entries and the index of this one.
    pub fn copy(&self, serial: usize) -> DictObj {
        DictObj {
            serial,
            entries: RefCell::new(Rc::clone(&self.entries.borrow())),
            index: RefCell::new(Rc::clone(&self.index.borrow())),
            shape: Cell::new(self.shape.get()),
        }
    }
    /// The entries, to change them: copied first if another Dict shares them.
    pub fn entries_mut(&self) -> std::cell::RefMut<'_, Vec<(Value, Value)>> {
        std::cell::RefMut::map(self.entries.borrow_mut(), Rc::make_mut)
    }
    /// The index, to change it: copied first if another Dict shares it.
    pub fn index_mut(&self) -> std::cell::RefMut<'_, DictIndex> {
        std::cell::RefMut::map(self.index.borrow_mut(), Rc::make_mut)
    }
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
        let k = index_key(&key)?;
        let shape = match &k {
            IndexKey::Str(t) => shape_after(self.shape.get(), t, str_hash(t)),
            _ => SHAPE_NONE,
        };
        self.shape.set(shape);
        self.index_mut().insert(k, n);
        self.entries_mut().push((key, value));
        Ok(())
    }
    pub fn remove(&self, i: usize) -> Result<(), String> {
        self.shape.set(SHAPE_NONE);
        self.entries_mut().remove(i);
        let mut index = DictIndex::default();
        for (i, (k, _)) in self.entries.borrow().iter().enumerate() {
            index.insert(index_key(k)?, i);
        }
        *self.index.borrow_mut() = Rc::new(index);
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
    /// The names of the variables of lvenv, kept with --trace (VProc::scope).
    pub scope: crate::module::Scope,
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
    // Drop a long list without recursion. The tail of a cell of a LazyList is a Ref whose value
    // is the next cell (or null), so a Ref that only this cell holds is gone through too:
    // otherwise dropping a LazyList of n elements recurses 2n deep (LList, Ref, LList, ...),
    // which overflows the stack of a browser at about 10,000 elements (consh/js).
    fn drop(&mut self) {
        let mut next = std::mem::replace(&mut self.tail, Value::Null);
        loop {
            next = match next {
                Value::LList(rc) => match Rc::try_unwrap(rc) {
                    Ok(mut cell) => std::mem::replace(&mut cell.tail, Value::Null),
                    Err(_) => break,
                },
                Value::Ref(rc) => match Rc::try_unwrap(rc) {
                    Ok(r) => r.value.into_inner(),
                    Err(_) => break,
                },
                _ => break,
            };
        }
    }
}

/// A local environment: the innermost binding first, as the Python list
/// `[(name, value), ...]`, but shared between Closures.
pub type LVEnv = Option<Rc<LVNode>>;

/// A node of an LVEnv: the value of one variable. The names of the variables are not kept: a
/// trace or a printed Closure finds them from the Body the LVEnv is for (module::Scope).
pub struct LVNode {
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

#[inline]
pub fn lv_push(env: &LVEnv, value: Value) -> LVEnv {
    Some(Rc::new(LVNode { value, next: env.clone() }))
}

/// env with the values bound to the parameters of a Function whose Body has this
/// BodyDef::frame: 0, a node for each value; 1, a new frame node with all of them; 2, added to
/// the frame node at the head of env, in place if nothing else holds the node (and its values),
/// otherwise in a copy of it.
#[inline(never)]
pub fn lv_bind(env: LVEnv, vals: impl Iterator<Item = Value>, frame: u8, size: u32) -> LVEnv {
    match frame {
        0 => {
            let mut env = env;
            for v in vals {
                env = Some(Rc::new(LVNode { value: v, next: env }));
            }
            env
        }
        1 => {
            let mut fr = Vec::with_capacity(size as usize);
            fr.extend(vals);
            Some(Rc::new(LVNode { value: Value::Frame(Rc::new(fr)), next: env }))
        }
        _ => {
            let mut node = env.expect("lv_bind: no frame node");
            if let Some(n) = Rc::get_mut(&mut node) {
                if let Value::Frame(fr) = &mut n.value {
                    match Rc::get_mut(fr) {
                        Some(v) => v.extend(vals),
                        None => {
                            let mut v = (**fr).clone();
                            v.extend(vals);
                            *fr = Rc::new(v);
                        }
                    }
                    return Some(node);
                }
                panic!("lv_bind: not a frame node");
            }
            let mut v = match &node.value {
                Value::Frame(fr) => (**fr).clone(),
                _ => panic!("lv_bind: not a frame node"),
            };
            v.extend(vals);
            Some(Rc::new(LVNode { value: Value::Frame(Rc::new(v)), next: node.next.clone() }))
        }
    }
}

/// The variable at slot k of the frame node at the head of env (module::CExpr::S).
#[inline]
pub fn lv_slot(env: &LVEnv, k: usize) -> Option<&Value> {
    match &env.as_ref()?.value {
        Value::Frame(fr) => fr.get(k),
        _ => None,
    }
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
        Value::Wasm(_) => "Wasm".into(),
        Value::Frame(_) => "Frame".into(),
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
        Value::Wasm(_) => "Wasm",
        Value::Raw(_) | Value::Null | Value::Frame(_) => return Err("__type_name__: unknown value".into()),
    })
}
