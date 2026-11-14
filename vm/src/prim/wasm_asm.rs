//! The assembler of __Wasm_compile__ (consh/SJS-WASM-PLAN-ja.md 5.3.1): the
//! WebAssembly text format (WAT) of a module of functions on numbers, made
//! into the binary format. It reads only this subset:
//!
//! * `(module (memory $m? min max?)? (func ...) ...)`: no import, table,
//!   global, start, data or elem. The module may have one memory of its own
//!   (consh/SJS-WASM-ARRAY-PLAN-ja.md, stage A1), exported as `__memory`, which
//!   the host fills with zeros before each call (and with the words of
//!   __Wasm_call_memory__), so that an exported function still depends on its
//!   arguments (and those words) only.
//! * `(func $name? (export "name")* (param $x? f64 ...)* (result f64)
//!   (local $y? T ...)* instruction*)`: parameters are f64, the result is one
//!   f64, locals are f64, i32 or i64.
//! * The instructions on f64, i32 and i64 (no f32), and `f64.load`,
//!   `f64.store`, `memory.size` and `memory.grow` on the memory, written flat
//!   (`block ... end`) or folded (`(f64.add (local.get 0) (local.get 1))`).
//!
//! The assembler adds the fuel: a mutable i64 global exported as `__fuel`,
//! taken by one at the start of each function and of each loop, the function
//! reaching `unreachable` when it is 0 (5.5), so that a loop that does not end
//! comes back to the VM. The host checks the types of the instructions when it
//! compiles the module (WebAssembly.Module).

use std::collections::HashMap;

/// The name of the exported fuel global.
pub const FUEL_EXPORT: &str = "__fuel";

/// The name of the exported memory.
pub const MEMORY_EXPORT: &str = "__memory";

/// A module made into binary, with the parameter counts of its exports and
/// whether it has a memory.
pub struct Assembled {
    pub bytes: Vec<u8>,
    pub exports: Vec<(String, usize)>,
    pub memory: bool,
    /// The most pages the memory may grow to (its max, or 65536, all that wasm32 can address).
    pub memory_max: u64,
}

#[derive(Clone, Debug)]
enum Sx {
    Atom(String, Pos),
    Str(String, Pos),
    List(Vec<Sx>, Pos),
}

#[derive(Clone, Copy, Debug)]
struct Pos {
    line: usize,
    col: usize,
}

fn err<T>(pos: Pos, msg: impl AsRef<str>) -> Result<T, String> {
    Err(format!("line {} col {}: {}", pos.line, pos.col, msg.as_ref()))
}

impl Sx {
    fn pos(&self) -> Pos {
        match self {
            Sx::Atom(_, p) | Sx::Str(_, p) | Sx::List(_, p) => *p,
        }
    }
    fn atom(&self) -> Option<&str> {
        match self {
            Sx::Atom(a, _) => Some(a),
            _ => None,
        }
    }
    /// The head atom of a list.
    fn head(&self) -> Option<&str> {
        match self {
            Sx::List(items, _) => items.first().and_then(|h| h.atom()),
            _ => None,
        }
    }
    fn describe(&self) -> String {
        match self {
            Sx::Atom(a, _) => a.clone(),
            Sx::Str(s, _) => format!("\"{}\"", s),
            Sx::List(_, _) => match self.head() {
                Some(h) => format!("({} ...)", h),
                None => "a list".into(),
            },
        }
    }
}

// ── Reading the text ──

/// The deepest nesting of lists in the text, and of blocks (block, loop, if) and folded
/// instructions in a function: the reader, the encoder and the dropping of the lists recurse
/// that deep, which would overflow the stack of the host for a text nested much deeper.
const MAX_NEST: usize = 10000;

fn read(text: &str) -> Result<Vec<Sx>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let (mut line, mut col) = (1, 1);
    // The lists being read: their items and positions.
    let mut stack: Vec<(Vec<Sx>, Pos)> = vec![(Vec::new(), Pos { line: 1, col: 1 })];
    macro_rules! advance {
        () => {{
            if chars[i] == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
            i += 1;
        }};
    }
    while i < chars.len() {
        let c = chars[i];
        let pos = Pos { line, col };
        if c.is_whitespace() {
            advance!();
        } else if c == ';' && chars.get(i + 1) == Some(&';') {
            while i < chars.len() && chars[i] != '\n' {
                advance!();
            }
        } else if c == '(' && chars.get(i + 1) == Some(&';') {
            // A block comment (; ... ;), which may be nested.
            let mut depth = 0;
            loop {
                if i >= chars.len() {
                    return err(pos, "unterminated block comment");
                }
                if chars[i] == '(' && chars.get(i + 1) == Some(&';') {
                    depth += 1;
                    advance!();
                    advance!();
                } else if chars[i] == ';' && chars.get(i + 1) == Some(&')') {
                    depth -= 1;
                    advance!();
                    advance!();
                    if depth == 0 {
                        break;
                    }
                } else {
                    advance!();
                }
            }
        } else if c == '(' {
            if stack.len() > MAX_NEST {
                return err(pos, format!("nested more than {} deep", MAX_NEST));
            }
            stack.push((Vec::new(), pos));
            advance!();
        } else if c == ')' {
            if stack.len() == 1 {
                return err(pos, "unexpected )");
            }
            let (items, p) = stack.pop().unwrap();
            stack.last_mut().unwrap().0.push(Sx::List(items, p));
            advance!();
        } else if c == '"' {
            advance!();
            let mut s = String::new();
            loop {
                if i >= chars.len() || chars[i] == '\n' {
                    return err(pos, "unterminated string");
                }
                let d = chars[i];
                advance!();
                if d == '"' {
                    break;
                }
                if d == '\\' {
                    if i >= chars.len() {
                        return err(pos, "unterminated string");
                    }
                    let e = chars[i];
                    advance!();
                    s.push(match e {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        '\'' => '\'',
                        _ => return err(pos, format!("unknown escape \\{} in a string", e)),
                    });
                } else {
                    s.push(d);
                }
            }
            stack.last_mut().unwrap().0.push(Sx::Str(s, pos));
        } else {
            let mut a = String::new();
            while i < chars.len() && !chars[i].is_whitespace() && !"()\";".contains(chars[i]) {
                a.push(chars[i]);
                advance!();
            }
            if a.is_empty() {
                // only a ';' that begins no comment stops an atom before its first character
                return err(pos, "unexpected ;");
            }
            stack.last_mut().unwrap().0.push(Sx::Atom(a, pos));
        }
    }
    if stack.len() > 1 {
        return err(stack.last().unwrap().1, "unclosed (");
    }
    Ok(stack.pop().unwrap().0)
}

// ── Numbers and the binary format ──

fn uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        let done = (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0);
        if done {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn name_bytes(out: &mut Vec<u8>, s: &str) {
    uleb(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

fn section(out: &mut Vec<u8>, id: u8, body: &[u8]) {
    out.push(id);
    uleb(out, body.len() as u64);
    out.extend_from_slice(body);
}

/// True if s is digits (hexadecimal ones with hex), with single _ between two of them.
fn digits_ok(s: &str, hex: bool) -> bool {
    let b = s.as_bytes();
    let digit = |c: u8| if hex { c.is_ascii_hexdigit() } else { c.is_ascii_digit() };
    !b.is_empty()
        && b.iter().enumerate().all(|(k, &c)| {
            digit(c) || (c == b'_' && k > 0 && k + 1 < b.len() && digit(b[k - 1]) && digit(b[k + 1]))
        })
}

/// An integer literal (decimal or 0x hexadecimal, with an optional sign and _
/// between digits) within [min, max].
fn parse_int(text: &str, min: i128, max: i128) -> Option<i128> {
    let (neg, digits) = match text.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let v = if let Some(h) = digits.strip_prefix("0x") {
        if !digits_ok(h, true) {
            return None;
        }
        i128::from_str_radix(&h.replace('_', ""), 16).ok()?
    } else {
        if !digits_ok(digits, false) {
            return None;
        }
        digits.replace('_', "").parse::<i128>().ok()?
    };
    let v = if neg { -v } else { v };
    if v < min || v > max {
        None
    } else {
        Some(v)
    }
}

/// An unsigned integer literal (an index, an offset, a number of pages: no sign) within [0, max].
fn parse_uint(text: &str, max: i128) -> Option<i128> {
    if text.starts_with('+') || text.starts_with('-') {
        return None;
    }
    parse_int(text, 0, max)
}

/// A float literal: decimal (as Rust reads it, rounded to the nearest), inf,
/// nan, with an optional sign.
fn parse_f64(text: &str) -> Option<f64> {
    // _ only between two digits
    let b = text.as_bytes();
    if b.iter().enumerate().any(|(k, &c)| c == b'_' && !(k > 0 && k + 1 < b.len() && b[k - 1].is_ascii_digit() && b[k + 1].is_ascii_digit())) {
        return None;
    }
    let t = text.replace('_', "");
    let (neg, body) = match t.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, t.strip_prefix('+').unwrap_or(&t)),
    };
    let v = match body {
        "inf" => f64::INFINITY,
        "nan" => f64::NAN,
        _ => {
            if body.starts_with("0x") || !body.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                return None;
            }
            if !body.chars().all(|c| c.is_ascii_digit() || ".eE+-".contains(c)) {
                return None;
            }
            body.parse::<f64>().ok()?
        }
    };
    Some(if neg { -v } else { v })
}

const F64: u8 = 0x7c;
const I32: u8 = 0x7f;
const I64: u8 = 0x7e;

fn valtype(s: &str) -> Option<u8> {
    match s {
        "f64" => Some(F64),
        "i32" => Some(I32),
        "i64" => Some(I64),
        _ => None,
    }
}

/// The opcode of an instruction without immediates.
fn plain_op(name: &str) -> Option<&'static [u8]> {
    Some(match name {
        "unreachable" => &[0x00],
        "nop" => &[0x01],
        "return" => &[0x0f],
        "drop" => &[0x1a],
        "select" => &[0x1b],
        "i32.eqz" => &[0x45],
        "i32.eq" => &[0x46],
        "i32.ne" => &[0x47],
        "i32.lt_s" => &[0x48],
        "i32.lt_u" => &[0x49],
        "i32.gt_s" => &[0x4a],
        "i32.gt_u" => &[0x4b],
        "i32.le_s" => &[0x4c],
        "i32.le_u" => &[0x4d],
        "i32.ge_s" => &[0x4e],
        "i32.ge_u" => &[0x4f],
        "i64.eqz" => &[0x50],
        "i64.eq" => &[0x51],
        "i64.ne" => &[0x52],
        "i64.lt_s" => &[0x53],
        "i64.lt_u" => &[0x54],
        "i64.gt_s" => &[0x55],
        "i64.gt_u" => &[0x56],
        "i64.le_s" => &[0x57],
        "i64.le_u" => &[0x58],
        "i64.ge_s" => &[0x59],
        "i64.ge_u" => &[0x5a],
        "f64.eq" => &[0x61],
        "f64.ne" => &[0x62],
        "f64.lt" => &[0x63],
        "f64.gt" => &[0x64],
        "f64.le" => &[0x65],
        "f64.ge" => &[0x66],
        "i32.clz" => &[0x67],
        "i32.ctz" => &[0x68],
        "i32.popcnt" => &[0x69],
        "i32.add" => &[0x6a],
        "i32.sub" => &[0x6b],
        "i32.mul" => &[0x6c],
        "i32.div_s" => &[0x6d],
        "i32.div_u" => &[0x6e],
        "i32.rem_s" => &[0x6f],
        "i32.rem_u" => &[0x70],
        "i32.and" => &[0x71],
        "i32.or" => &[0x72],
        "i32.xor" => &[0x73],
        "i32.shl" => &[0x74],
        "i32.shr_s" => &[0x75],
        "i32.shr_u" => &[0x76],
        "i32.rotl" => &[0x77],
        "i32.rotr" => &[0x78],
        "i64.clz" => &[0x79],
        "i64.ctz" => &[0x7a],
        "i64.popcnt" => &[0x7b],
        "i64.add" => &[0x7c],
        "i64.sub" => &[0x7d],
        "i64.mul" => &[0x7e],
        "i64.div_s" => &[0x7f],
        "i64.div_u" => &[0x80],
        "i64.rem_s" => &[0x81],
        "i64.rem_u" => &[0x82],
        "i64.and" => &[0x83],
        "i64.or" => &[0x84],
        "i64.xor" => &[0x85],
        "i64.shl" => &[0x86],
        "i64.shr_s" => &[0x87],
        "i64.shr_u" => &[0x88],
        "i64.rotl" => &[0x89],
        "i64.rotr" => &[0x8a],
        "f64.abs" => &[0x99],
        "f64.neg" => &[0x9a],
        "f64.ceil" => &[0x9b],
        "f64.floor" => &[0x9c],
        "f64.trunc" => &[0x9d],
        "f64.nearest" => &[0x9e],
        "f64.sqrt" => &[0x9f],
        "f64.add" => &[0xa0],
        "f64.sub" => &[0xa1],
        "f64.mul" => &[0xa2],
        "f64.div" => &[0xa3],
        "f64.min" => &[0xa4],
        "f64.max" => &[0xa5],
        "f64.copysign" => &[0xa6],
        "i32.wrap_i64" => &[0xa7],
        "i32.trunc_f64_s" => &[0xaa],
        "i32.trunc_f64_u" => &[0xab],
        "i64.extend_i32_s" => &[0xac],
        "i64.extend_i32_u" => &[0xad],
        "i64.trunc_f64_s" => &[0xb0],
        "i64.trunc_f64_u" => &[0xb1],
        "f64.convert_i32_s" => &[0xb7],
        "f64.convert_i32_u" => &[0xb8],
        "f64.convert_i64_s" => &[0xb9],
        "f64.convert_i64_u" => &[0xba],
        "i64.reinterpret_f64" => &[0xbd],
        "f64.reinterpret_i64" => &[0xbf],
        "i32.extend8_s" => &[0xc0],
        "i32.extend16_s" => &[0xc1],
        "i64.extend8_s" => &[0xc2],
        "i64.extend16_s" => &[0xc3],
        "i64.extend32_s" => &[0xc4],
        "i32.trunc_sat_f64_s" => &[0xfc, 0x02],
        "i32.trunc_sat_f64_u" => &[0xfc, 0x03],
        "i64.trunc_sat_f64_s" => &[0xfc, 0x06],
        "i64.trunc_sat_f64_u" => &[0xfc, 0x07],
        _ => return None,
    })
}

/// The code that takes one from the fuel (global 0), reaching unreachable when it is 0.
const FUEL_CHECK: &[u8] = &[
    0x23, 0x00, 0x50, 0x04, 0x40, 0x00, 0x0b, // if fuel == 0 then unreachable
    0x23, 0x00, 0x42, 0x01, 0x7d, 0x24, 0x00, // fuel = fuel - 1
];

// ── Functions ──

struct Func {
    pos: Pos,
    exports: Vec<(String, Pos)>,
    params: Vec<Option<String>>,
    locals: Vec<(Option<String>, u8)>,
    body: Vec<Sx>,
}

/// What an instruction sequence is encoded with.
struct Ctx<'a> {
    funcs: &'a HashMap<String, usize>,
    nfuncs: usize,
    locals: HashMap<String, usize>,
    nlocals: usize,
    /// The labels of the blocks around, innermost last.
    labels: Vec<Option<String>>,
    /// The blocks and folded instructions being encoded (at most MAX_NEST).
    depth: usize,
}

impl Ctx<'_> {
    fn enter(&mut self, pos: Pos) -> Result<(), String> {
        if self.depth >= MAX_NEST {
            return err(pos, format!("nested more than {} deep", MAX_NEST));
        }
        self.depth += 1;
        Ok(())
    }
}

impl<'a> Ctx<'a> {
    fn local(&self, x: &Sx) -> Result<u64, String> {
        match x.atom() {
            Some(a) if a.starts_with('$') => match self.locals.get(a) {
                Some(&i) => Ok(i as u64),
                None => err(x.pos(), format!("unknown local {}", a)),
            },
            Some(a) => match parse_uint(a, self.nlocals as i128 - 1) {
                Some(i) => Ok(i as u64),
                None => err(x.pos(), format!("unknown local {}", a)),
            },
            None => err(x.pos(), format!("expected a local, got {}", x.describe())),
        }
    }
    fn label(&self, x: &Sx) -> Result<u64, String> {
        match x.atom() {
            Some(a) if a.starts_with('$') => {
                match self.labels.iter().rev().position(|l| l.as_deref() == Some(a)) {
                    Some(d) => Ok(d as u64),
                    None => err(x.pos(), format!("unknown label {}", a)),
                }
            }
            // The function body is the outermost target, at depth labels.len().
            Some(a) => match parse_uint(a, self.labels.len() as i128) {
                Some(d) => Ok(d as u64),
                None => err(x.pos(), format!("unknown label {}", a)),
            },
            None => err(x.pos(), format!("expected a label, got {}", x.describe())),
        }
    }
    fn func(&self, x: &Sx) -> Result<u64, String> {
        match x.atom() {
            Some(a) if a.starts_with('$') => match self.funcs.get(a) {
                Some(&i) => Ok(i as u64),
                None => err(x.pos(), format!("unknown function {}", a)),
            },
            Some(a) => match parse_uint(a, self.nfuncs as i128 - 1) {
                Some(i) => Ok(i as u64),
                None => err(x.pos(), format!("unknown function {}", a)),
            },
            None => err(x.pos(), format!("expected a function, got {}", x.describe())),
        }
    }
}

/// The number of immediates an instruction takes.
fn immediates(op: &str) -> usize {
    match op {
        "local.get" | "local.set" | "local.tee" | "br" | "br_if" | "call" | "i32.const" | "i64.const" | "f64.const" => 1,
        _ => 0,
    }
}

/// An instruction on the memory: its opcode, or for f64.load and f64.store,
/// the opcode of an instruction with a memarg.
fn memory_op(op: &str) -> Option<(&'static [u8], bool)> {
    match op {
        "f64.load" => Some((&[0x2b], true)),
        "f64.store" => Some((&[0x39], true)),
        "memory.size" => Some((&[0x3f, 0x00], false)),
        "memory.grow" => Some((&[0x40, 0x00], false)),
        _ => None,
    }
}

/// The optional offset=N and align=N of a load or store, from items[*i]
/// (align is in bytes, 1, 2, 4 or 8, and 8 by default).
fn memarg(items: &[Sx], i: &mut usize, out: &mut Vec<u8>) -> Result<(), String> {
    let mut offset: u64 = 0;
    let mut align: u64 = 3;
    while let Some(Sx::Atom(a, pos)) = items.get(*i) {
        if let Some(v) = a.strip_prefix("offset=") {
            match parse_uint(v, u32::MAX as i128) {
                Some(n) => offset = n as u64,
                None => return err(*pos, format!("bad offset {}", v)),
            }
        } else if let Some(v) = a.strip_prefix("align=") {
            align = match v {
                "1" => 0,
                "2" => 1,
                "4" => 2,
                "8" => 3,
                _ => return err(*pos, format!("bad align {} (1, 2, 4 or 8)", v)),
            };
        } else {
            break;
        }
        *i += 1;
    }
    uleb(out, align);
    uleb(out, offset);
    Ok(())
}

fn encode_immediate(ctx: &Ctx, op: &str, x: &Sx, out: &mut Vec<u8>) -> Result<(), String> {
    match op {
        "local.get" => {
            out.push(0x20);
            uleb(out, ctx.local(x)?);
        }
        "local.set" => {
            out.push(0x21);
            uleb(out, ctx.local(x)?);
        }
        "local.tee" => {
            out.push(0x22);
            uleb(out, ctx.local(x)?);
        }
        "br" => {
            out.push(0x0c);
            uleb(out, ctx.label(x)?);
        }
        "br_if" => {
            out.push(0x0d);
            uleb(out, ctx.label(x)?);
        }
        "call" => {
            out.push(0x10);
            uleb(out, ctx.func(x)?);
        }
        "i32.const" => {
            let v = x.atom().and_then(|a| parse_int(a, i32::MIN as i128, u32::MAX as i128));
            match v {
                Some(v) => {
                    out.push(0x41);
                    sleb(out, v as u32 as i32 as i64);
                }
                None => return err(x.pos(), format!("expected an i32, got {}", x.describe())),
            }
        }
        "i64.const" => {
            let v = x.atom().and_then(|a| parse_int(a, i64::MIN as i128, u64::MAX as i128));
            match v {
                Some(v) => {
                    out.push(0x42);
                    sleb(out, v as u64 as i64);
                }
                None => return err(x.pos(), format!("expected an i64, got {}", x.describe())),
            }
        }
        "f64.const" => match x.atom().and_then(parse_f64) {
            Some(v) => {
                out.push(0x44);
                out.extend_from_slice(&v.to_le_bytes());
            }
            None => return err(x.pos(), format!("expected an f64, got {}", x.describe())),
        },
        _ => unreachable!(),
    }
    Ok(())
}

/// The optional $label and (result t) of a block, from items[*i].
fn block_head(items: &[Sx], i: &mut usize) -> Result<(Option<String>, u8), String> {
    let mut label = None;
    if let Some(a) = items.get(*i).and_then(|x| x.atom()) {
        if a.starts_with('$') {
            label = Some(a.to_string());
            *i += 1;
        }
    }
    let mut bt = 0x40;
    if let Some(x) = items.get(*i) {
        match x.head() {
            Some("result") => {
                let Sx::List(r, _) = x else { unreachable!() };
                if r.len() != 2 {
                    return err(x.pos(), "a block has at most one result");
                }
                bt = match r[1].atom().and_then(valtype) {
                    Some(t) => t,
                    None => return err(r[1].pos(), format!("unknown type {}", r[1].describe())),
                };
                *i += 1;
            }
            Some("param") | Some("type") => return err(x.pos(), "a block cannot take parameters"),
            _ => {}
        }
    }
    Ok((label, bt))
}

/// Skip the $label after end or else (word) at items[*i], if any: it must be the label of the
/// block.
fn end_label(items: &[Sx], i: &mut usize, label: &Option<String>, word: &str) -> Result<(), String> {
    if let Some(Sx::Atom(l, pos)) = items.get(*i) {
        if l.starts_with('$') {
            if label.as_deref() != Some(l.as_str()) {
                return err(*pos, format!("{} {} does not match the label of the block", word, l));
            }
            *i += 1;
        }
    }
    Ok(())
}

/// Encode the flat or folded instructions items[*i..] until one of the atoms
/// in stop (returned, None at the end of items).
fn encode_seq(ctx: &mut Ctx, items: &[Sx], i: &mut usize, stop: &[&str], out: &mut Vec<u8>) -> Result<Option<String>, String> {
    while *i < items.len() {
        let x = &items[*i];
        match x {
            Sx::Atom(a, pos) => {
                if stop.contains(&a.as_str()) {
                    *i += 1;
                    return Ok(Some(a.clone()));
                }
                *i += 1;
                match a.as_str() {
                    "block" | "loop" | "if" => {
                        let (label, bt) = block_head(items, i)?;
                        out.push(match a.as_str() {
                            "block" => 0x02,
                            "loop" => 0x03,
                            _ => 0x04,
                        });
                        out.push(bt);
                        if a == "loop" {
                            out.extend_from_slice(FUEL_CHECK);
                        }
                        ctx.labels.push(label.clone());
                        ctx.enter(*pos)?;
                        let stops: &[&str] = if a == "if" { &["else", "end"] } else { &["end"] };
                        let mut found = encode_seq(ctx, items, i, stops, out)?;
                        if found.as_deref() == Some("else") {
                            // an optional $label after else, the label of the if
                            end_label(items, i, &label, "else")?;
                            out.push(0x05);
                            found = encode_seq(ctx, items, i, &["end"], out)?;
                        }
                        if found.is_none() {
                            return err(*pos, format!("{} without end", a));
                        }
                        ctx.depth -= 1;
                        ctx.labels.pop();
                        // an optional $label after end, the label of the block
                        end_label(items, i, &label, "end")?;
                        out.push(0x0b);
                    }
                    "else" | "end" => return err(*pos, format!("unexpected {}", a)),
                    op if memory_op(op).is_some() => {
                        let (code, has_memarg) = memory_op(op).unwrap();
                        out.extend_from_slice(code);
                        if has_memarg {
                            memarg(items, i, out)?;
                        }
                    }
                    op if immediates(op) == 1 => match items.get(*i) {
                        Some(imm) => {
                            *i += 1;
                            encode_immediate(ctx, op, imm, out)?;
                        }
                        None => return err(*pos, format!("{} needs an operand", op)),
                    },
                    op => match plain_op(op) {
                        Some(code) => out.extend_from_slice(code),
                        None => return err(*pos, format!("unknown instruction {}", op)),
                    },
                }
            }
            Sx::List(_, _) => {
                *i += 1;
                encode_folded(ctx, x, out)?;
            }
            Sx::Str(_, pos) => return err(*pos, format!("unexpected {}", x.describe())),
        }
    }
    Ok(None)
}

/// Encode a folded instruction (op immediates... operands...).
fn encode_folded(ctx: &mut Ctx, x: &Sx, out: &mut Vec<u8>) -> Result<(), String> {
    let Sx::List(_, pos) = x else { unreachable!() };
    ctx.enter(*pos)?;
    let r = encode_folded_in(ctx, x, out);
    ctx.depth -= 1;
    r
}

fn encode_folded_in(ctx: &mut Ctx, x: &Sx, out: &mut Vec<u8>) -> Result<(), String> {
    let Sx::List(items, pos) = x else { unreachable!() };
    let op = match items.first().and_then(|h| h.atom()) {
        Some(op) => op,
        None => return err(*pos, "expected an instruction"),
    };
    let mut i = 1;
    match op {
        "block" | "loop" => {
            let (label, bt) = block_head(items, &mut i)?;
            out.push(if op == "block" { 0x02 } else { 0x03 });
            out.push(bt);
            if op == "loop" {
                out.extend_from_slice(FUEL_CHECK);
            }
            ctx.labels.push(label);
            if let Some(s) = encode_seq(ctx, items, &mut i, &[], out)? {
                return err(*pos, format!("unexpected {}", s));
            }
            ctx.labels.pop();
            out.push(0x0b);
        }
        "if" => {
            let (label, bt) = block_head(items, &mut i)?;
            // The condition: the folded instructions before (then ...).
            while i < items.len() && items[i].head() != Some("then") {
                match &items[i] {
                    y @ Sx::List(_, _) => encode_folded(ctx, y, out)?,
                    y => return err(y.pos(), format!("expected (then ...), got {}", y.describe())),
                }
                i += 1;
            }
            let then = match items.get(i) {
                Some(Sx::List(t, _)) => t,
                _ => return err(*pos, "if without (then ...)"),
            };
            i += 1;
            out.push(0x04);
            out.push(bt);
            ctx.labels.push(label);
            let mut j = 1;
            if let Some(s) = encode_seq(ctx, then, &mut j, &[], out)? {
                return err(*pos, format!("unexpected {}", s));
            }
            if let Some(e) = items.get(i) {
                match e {
                    Sx::List(el, _) if e.head() == Some("else") => {
                        out.push(0x05);
                        let mut j = 1;
                        if let Some(s) = encode_seq(ctx, el, &mut j, &[], out)? {
                            return err(*pos, format!("unexpected {}", s));
                        }
                    }
                    _ => return err(e.pos(), format!("expected (else ...), got {}", e.describe())),
                }
                i += 1;
            }
            if let Some(extra) = items.get(i) {
                return err(extra.pos(), format!("unexpected {}", extra.describe()));
            }
            ctx.labels.pop();
            out.push(0x0b);
        }
        op if memory_op(op).is_some() => {
            let (code, has_memarg) = memory_op(op).unwrap();
            let mut arg = Vec::new();
            if has_memarg {
                memarg(items, &mut i, &mut arg)?;
            }
            for y in &items[i..] {
                match y {
                    Sx::List(_, _) => encode_folded(ctx, y, out)?,
                    _ => return err(y.pos(), format!("expected a folded instruction, got {}", y.describe())),
                }
            }
            out.extend_from_slice(code);
            out.extend_from_slice(&arg);
        }
        _ => {
            let imm = if immediates(op) == 1 {
                match items.get(1) {
                    Some(y) if !matches!(y, Sx::List(_, _)) => {
                        i = 2;
                        Some(y)
                    }
                    _ => return err(*pos, format!("{} needs an operand", op)),
                }
            } else {
                if plain_op(op).is_none() {
                    return err(items[0].pos(), format!("unknown instruction {}", op));
                }
                None
            };
            for y in &items[i..] {
                match y {
                    Sx::List(_, _) => encode_folded(ctx, y, out)?,
                    _ => return err(y.pos(), format!("expected a folded instruction, got {}", y.describe())),
                }
            }
            match imm {
                Some(y) => encode_immediate(ctx, op, y, out)?,
                None => out.extend_from_slice(plain_op(op).unwrap()),
            }
        }
    }
    Ok(())
}

/// Read a (func ...) into its parts.
fn read_func(items: &[Sx], pos: Pos) -> Result<(Option<String>, Func), String> {
    let mut i = 1;
    let mut name = None;
    if let Some(a) = items.get(i).and_then(|x| x.atom()) {
        if a.starts_with('$') {
            name = Some(a.to_string());
            i += 1;
        }
    }
    let mut f = Func { pos, exports: Vec::new(), params: Vec::new(), locals: Vec::new(), body: Vec::new() };
    let mut results = 0;
    // The parts in their order: export, param, result, local; then the instructions.
    let mut stage = 0;
    while i < items.len() {
        let x = &items[i];
        let (part, order) = match x.head() {
            Some("export") => ("export", 0),
            Some("param") => ("param", 1),
            Some("result") => ("result", 2),
            Some("local") => ("local", 3),
            Some(h @ ("import" | "type")) => return err(x.pos(), format!("({} ...) is not allowed in a func", h)),
            _ => break,
        };
        if order < stage {
            return err(x.pos(), format!("({} ...) is out of order in a func", part));
        }
        stage = order;
        let Sx::List(p, _) = x else { unreachable!() };
        match part {
            "export" => match p.as_slice() {
                [_, Sx::Str(s, sp)] => f.exports.push((s.clone(), *sp)),
                _ => return err(x.pos(), "expected (export \"name\")"),
            },
            "param" | "local" => {
                let named = p.get(1).and_then(|y| y.atom()).map_or(false, |a| a.starts_with('$'));
                let (names, types): (Vec<Option<String>>, &[Sx]) = if named {
                    if p.len() != 3 {
                        return err(x.pos(), format!("expected ({} $name type)", part));
                    }
                    (vec![p[1].atom().map(|a| a.to_string())], &p[2..])
                } else {
                    (vec![None; p.len() - 1], &p[1..])
                };
                for (n, t) in names.into_iter().zip(types) {
                    let ty = match t.atom().and_then(valtype) {
                        Some(ty) => ty,
                        None => return err(t.pos(), format!("unknown type {}", t.describe())),
                    };
                    if part == "param" {
                        if ty != F64 {
                            return err(t.pos(), "a parameter must be f64");
                        }
                        f.params.push(n);
                    } else {
                        f.locals.push((n, ty));
                    }
                }
            }
            _ => {
                if p.len() != 2 || p[1].atom() != Some("f64") {
                    return err(x.pos(), "the result must be one f64");
                }
                results += 1;
            }
        }
        i += 1;
    }
    if results != 1 {
        return err(pos, "a func must have exactly one (result f64)");
    }
    f.body = items[i..].to_vec();
    Ok((name, f))
}

/// Assemble the text of a module.
pub fn assemble(text: &str) -> Result<Assembled, String> {
    let top = read(text)?;
    let module = match top.as_slice() {
        [m] if m.head() == Some("module") => m,
        [] => return Err("expected (module ...)".into()),
        [x, ..] if x.head() != Some("module") => return err(x.pos(), format!("expected (module ...), got {}", x.describe())),
        [_, x, ..] => return err(x.pos(), "expected one (module ...)"),
        _ => unreachable!(),
    };
    let Sx::List(fields, _) = module else { unreachable!() };
    let mut start = 1;
    if let Some(a) = fields.get(1).and_then(|x| x.atom()) {
        if a.starts_with('$') {
            start = 2;
        }
    }
    let mut funcs = Vec::new();
    let mut names = HashMap::new();
    // The memory: (min, max)
    let mut memory: Option<(u64, Option<u64>)> = None;
    for x in &fields[start..] {
        match x.head() {
            Some("memory") => {
                let Sx::List(items, p) = x else { unreachable!() };
                if memory.is_some() {
                    return err(*p, "a module has at most one memory");
                }
                let mut j = 1;
                if let Some(a) = items.get(j).and_then(|y| y.atom()) {
                    if a.starts_with('$') {
                        j += 1;
                    }
                }
                let limits: Vec<Option<i128>> = items[j..].iter().map(|y| y.atom().and_then(|a| parse_uint(a, 65536))).collect();
                memory = Some(match limits.as_slice() {
                    [Some(min)] => (*min as u64, None),
                    [Some(min), Some(max)] if max >= min => (*min as u64, Some(*max as u64)),
                    _ => return err(*p, "expected (memory min max?), the numbers of pages from 0 to 65536 (no import or export)"),
                });
            }
            Some("func") => {
                let Sx::List(items, p) = x else { unreachable!() };
                let (name, f) = read_func(items, *p)?;
                if let Some(n) = name {
                    if names.insert(n.clone(), funcs.len()).is_some() {
                        return err(x.pos(), format!("function {} is defined twice", n));
                    }
                }
                funcs.push(f);
            }
            Some(h) => return err(x.pos(), format!("({} ...) is not allowed in a module (only func and memory)", h)),
            None => return err(x.pos(), format!("expected (func ...), got {}", x.describe())),
        }
    }
    // Types: one for each number of parameters.
    let mut types: Vec<usize> = Vec::new();
    let mut type_of = Vec::new();
    for f in &funcs {
        let n = f.params.len();
        let t = match types.iter().position(|&m| m == n) {
            Some(t) => t,
            None => {
                types.push(n);
                types.len() - 1
            }
        };
        type_of.push(t);
    }
    let mut exports: Vec<(String, usize)> = Vec::new();
    let mut export_items = Vec::new();
    for (k, f) in funcs.iter().enumerate() {
        for (e, p) in &f.exports {
            if e == FUEL_EXPORT || e == MEMORY_EXPORT {
                return err(*p, format!("the export name {} is reserved", e));
            }
            if exports.iter().any(|(n, _)| n == e) {
                return err(*p, format!("the export name \"{}\" is used twice", e));
            }
            exports.push((e.clone(), f.params.len()));
            export_items.push((e.clone(), k));
        }
    }
    let mut codes = Vec::new();
    for f in &funcs {
        let mut locals = HashMap::new();
        for (k, n) in f.params.iter().map(|n| n.as_ref()).chain(f.locals.iter().map(|(n, _)| n.as_ref())).enumerate() {
            if let Some(n) = n {
                if locals.insert(n.clone(), k).is_some() {
                    return err(f.pos, format!("local {} is declared twice", n));
                }
            }
        }
        let mut ctx = Ctx { funcs: &names, nfuncs: funcs.len(), locals, nlocals: f.params.len() + f.locals.len(), labels: Vec::new(), depth: 0 };
        let mut code = Vec::new();
        // Locals, in runs of the same type.
        let mut runs: Vec<(u64, u8)> = Vec::new();
        for (_, t) in &f.locals {
            match runs.last_mut() {
                Some((c, rt)) if rt == t => *c += 1,
                _ => runs.push((1, *t)),
            }
        }
        uleb(&mut code, runs.len() as u64);
        for (c, t) in runs {
            uleb(&mut code, c);
            code.push(t);
        }
        code.extend_from_slice(FUEL_CHECK);
        let mut i = 0;
        if let Some(s) = encode_seq(&mut ctx, &f.body, &mut i, &[], &mut code)? {
            return err(f.pos, format!("unexpected {}", s));
        }
        code.push(0x0b);
        codes.push(code);
    }
    let mut out = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let mut body = Vec::new();
    uleb(&mut body, types.len() as u64);
    for n in &types {
        body.push(0x60);
        uleb(&mut body, *n as u64);
        body.extend(std::iter::repeat(F64).take(*n));
        body.extend_from_slice(&[0x01, F64]);
    }
    section(&mut out, 1, &body);
    body.clear();
    uleb(&mut body, funcs.len() as u64);
    for t in &type_of {
        uleb(&mut body, *t as u64);
    }
    section(&mut out, 3, &body);
    if let Some((min, max)) = memory {
        body.clear();
        body.push(0x01);
        match max {
            None => {
                body.push(0x00);
                uleb(&mut body, min);
            }
            Some(m) => {
                body.push(0x01);
                uleb(&mut body, min);
                uleb(&mut body, m);
            }
        }
        section(&mut out, 5, &body);
    }
    // The fuel: a mutable i64 that starts at 0.
    section(&mut out, 6, &[0x01, I64, 0x01, 0x42, 0x00, 0x0b]);
    body.clear();
    uleb(&mut body, export_items.len() as u64 + 1 + memory.is_some() as u64);
    for (e, k) in &export_items {
        name_bytes(&mut body, e);
        body.push(0x00);
        uleb(&mut body, *k as u64);
    }
    name_bytes(&mut body, FUEL_EXPORT);
    body.extend_from_slice(&[0x03, 0x00]);
    if memory.is_some() {
        name_bytes(&mut body, MEMORY_EXPORT);
        body.extend_from_slice(&[0x02, 0x00]);
    }
    section(&mut out, 7, &body);
    body.clear();
    uleb(&mut body, codes.len() as u64);
    for c in &codes {
        uleb(&mut body, c.len() as u64);
        body.extend_from_slice(c);
    }
    section(&mut out, 10, &body);
    let memory_max = memory.and_then(|(_, max)| max).unwrap_or(65536);
    Ok(Assembled { bytes: out, exports, memory: memory.is_some(), memory_max })
}
