//! Word expansion, as consh_expand.se (PLAN-ja.md 1.2 and 1.3).
//!
//! A word is first made into parts, one for each word element:
//!   Plain   a String outside quotes (glob applies)
//!   Quoted  a String in quotes
//!   Braced  the String value of ${x}: an argument even when empty
//!   Leaves  the LazyList value of $x: one word for each leaf
//!   List    the LazyList value of ${x} or $( ): passed as it is to functions
//!           and builtins (not active for the leaves of ${-word})
//!   Func    a function value of $x or ${x} (stage 20): one value, only when
//!           it is the only part of the word
//! A leaf is a String reached by going down nested LazyLists.
//!
//! A function value cannot be made a String: in a word with other parts, in
//! "...", as an argument of an external command or as a leaf of a LazyList it
//! is an error. Going down a LazyList (leaves_into) always has a Limit, the
//! byte limit of an external command or a guard that only gives that error,
//! at the position the ConMa version gives it.

use crate::ast::{Pos, Sexp};
use crate::eval::Ctx;
use crate::glob;
use crate::value::{LazyList, Value, R};

#[derive(Clone, PartialEq, Copy)]
pub enum Kind {
    Plain,
    Quoted,
    Braced,
    Leaves,
    List,
    Func,
}

#[derive(Clone)]
pub struct Part {
    pub kind: Kind,
    pub value: Value,
    /// false for the leaves of ${-word}: no glob.
    pub active: bool,
}

impl Part {
    fn new(kind: Kind, value: Value) -> Part {
        Part { kind, value, active: true }
    }
    fn is_list(&self) -> bool {
        matches!(self.kind, Kind::Leaves | Kind::List)
    }
    fn text(&self) -> &str {
        self.value.as_str().unwrap_or("")
    }
}

pub const FUNC_STRING: &str = "a function value cannot be made into a string";

/// The byte limit of the arguments of an external command: each leaf takes
/// its number of characters + 1 from left (Arg_charge), and going below 0
/// is an error.
pub struct Limit {
    pub left: i64,
    pub max: i64,
    pub pos: Pos,
    /// A guard: no byte limit, only the error for a function value (Limit_guard).
    pub unlimited: bool,
}

impl Limit {
    pub fn new(max: i64, pos: Pos) -> Limit {
        Limit { left: max, max, pos, unlimited: false }
    }
    pub fn guard(pos: Pos) -> Limit {
        Limit { left: 0, max: 0, pos, unlimited: true }
    }
}

pub fn over_message(max: i64) -> String {
    format!("the arguments of an external command exceed ARG_MAX ({} bytes)", max)
}

fn charge(sh: &Ctx, limit: &mut Limit, s: &str) -> R<()> {
    if let l @ Limit { unlimited: false, .. } = limit {
        l.left -= s.chars().count() as i64 + 1;
        if l.left < 0 {
            return Err(sh.error(l.pos, &over_message(l.max)));
        }
    }
    Ok(())
}

fn word_elems(word: &Sexp) -> &[Sexp] {
    word.children()
}

// ── Words of external commands ──

/// The words of a word expanded as words of an external command: Strings,
/// or a function value if it is the word. A word without LazyLists that
/// becomes empty and is not quoted disappears; the leaves of a LazyList stay
/// even when empty.
pub fn fields(sh: &Ctx, word: &Sexp) -> R<Vec<Value>> {
    fields_in(sh, word, &mut Limit::guard(word.pos()))
}

pub fn fields_in(sh: &Ctx, word: &Sexp, limit: &mut Limit) -> R<Vec<Value>> {
    let parts = parts(sh, word_elems(word))?;
    parts_fields_in(sh, &parts, limit, false)
}

fn strings(v: Vec<String>) -> Vec<Value> {
    v.into_iter().map(Value::string).collect()
}

/// The words of the parts with glob applied. If a pattern matches nothing,
/// the word stays, or disappears if null_glob (${-word}).
pub fn parts_fields_in(sh: &Ctx, parts: &[Part], limit: &mut Limit, null_glob: bool) -> R<Vec<Value>> {
    if let Some(f) = single_func(parts) {
        return Ok(f);
    }
    if let Some((kind, value)) = parts_single(parts) {
        if let Some(f) = single_fields(kind, value) {
            return Ok(strings(f));
        }
    }
    // Candidates: (text, pattern, has a wildcard).
    let mut alts: Vec<(String, String, bool)> = vec![(String::new(), String::new(), false)];
    let mut quoted = false;
    let mut has_list = false;
    for part in parts {
        let is_quoted = part.kind == Kind::Quoted;
        quoted |= is_quoted;
        has_list |= part.is_list();
        let active = !is_quoted && part.active;
        let opts: Vec<String> = if part.is_list() {
            value_leaves_in(sh, &part.value, limit)?
        } else {
            vec![part.text().to_string()]
        };
        let opts: Vec<(String, String, bool)> = opts
            .into_iter()
            .map(|o| {
                if active {
                    let piece = glob::escape_active(&o);
                    let magic = glob::has_meta(&o);
                    (o, piece, magic)
                } else {
                    let piece = glob::escape(&o);
                    (o, piece, false)
                }
            })
            .collect();
        let mut next = Vec::with_capacity(alts.len() * opts.len());
        for (t, p, m) in &alts {
            for (ot, op, om) in &opts {
                next.push((format!("{}{}", t, ot), format!("{}{}", p, op), *m || *om));
            }
        }
        alts = next;
    }
    let keep = quoted || has_list;
    let mut out = Vec::new();
    for (text, pattern, magic) in alts {
        if text.is_empty() && !keep {
            continue;
        }
        if magic {
            let matches = glob::expand(&pattern, &sh.cwd());
            if !matches.is_empty() {
                out.extend(matches);
                continue;
            }
            if null_glob {
                continue;
            }
        }
        out.push(text);
    }
    Ok(strings(out))
}

/// The function value if it is the only part.
fn single_func(parts: &[Part]) -> Option<Vec<Value>> {
    match parts {
        [p] if p.kind == Kind::Func => Some(vec![p.value.clone()]),
        _ => None,
    }
}

/// The kind and value of the parts if there is one part and it is not a
/// LazyList (nor a function value).
fn parts_single(parts: &[Part]) -> Option<(Kind, &str)> {
    if parts.len() == 1 && !parts[0].is_list() && parts[0].kind != Kind::Func {
        Some((parts[0].kind, parts[0].text()))
    } else {
        None
    }
}

/// The words of one part that is not a LazyList, or None if glob applies.
fn single_fields(kind: Kind, value: &str) -> Option<Vec<String>> {
    if kind == Kind::Quoted {
        return Some(vec![value.to_string()]);
    }
    if value.is_empty() {
        return Some(Vec::new());
    }
    if glob::has_meta(value) {
        return None;
    }
    Some(vec![value.to_string()])
}

// ── Words of functions and builtins ──

/// The values of a word expanded as a word of a function or a builtin. With
/// a LazyList of $x, the leaves; otherwise with a LazyList of ${x}, one
/// LazyList; otherwise a word with quotes or ${x} is one String, and other
/// words disappear when empty.
pub fn args(sh: &Ctx, word: &Sexp) -> R<Vec<Value>> {
    let parts = parts(sh, word_elems(word))?;
    let guard = &mut Limit::guard(word.pos());
    if let Some(f) = single_func(&parts) {
        return Ok(f);
    }
    if let Some((kind, value)) = parts_single(&parts) {
        if kind == Kind::Braced && value.is_empty() {
            return Ok(vec![Value::str("")]);
        }
        if let Some(f) = single_fields(kind, value) {
            return Ok(strings(f));
        }
        return parts_fields_in(sh, &parts, guard, false);
    }
    let n_leaves = parts.iter().filter(|p| p.kind == Kind::Leaves).count();
    if n_leaves > 0 {
        return parts_fields_in(sh, &parts, guard, false);
    }
    let n_list = parts.iter().filter(|p| p.kind == Kind::List).count();
    if n_list > 0 {
        return Ok(vec![parts_list_value(sh, &parts, n_list, guard)?]);
    }
    let n_braced = parts.iter().filter(|p| p.kind == Kind::Braced).count();
    let s: String = parts.iter().map(|p| p.text()).collect();
    if s.is_empty() && n_braced > 0 {
        return Ok(vec![Value::str("")]);
    }
    parts_fields_in(sh, &parts, guard, false)
}

// ── The right side of a variable definition ──

/// The value of the right side of a definition: the value nodes (a word or
/// an array), or "" if there is none.
pub fn def_value(sh: &Ctx, values: &[Sexp]) -> R<Value> {
    let v = match values.first() {
        None => return Ok(Value::str("")),
        Some(v) => v,
    };
    if v.tag() == "fun" {
        return Ok(crate::eval::func_value(sh, v.children()));
    }
    if v.tag() == "array" {
        let mut items = Vec::new();
        for w in v.children() {
            items.extend(fields(sh, w)?);
        }
        return Ok(Value::List(LazyList::from_vec(items)));
    }
    value(sh, v)
}

/// The value of a word of the right side of a definition. With a LazyList of
/// $x, its one leaf word ("" if none, an error if more); otherwise with a
/// LazyList of ${x}, a LazyList; otherwise a String.
pub fn value(sh: &Ctx, word: &Sexp) -> R<Value> {
    let parts = parts(sh, word_elems(word))?;
    value_parts(sh, word.pos(), &parts)
}

pub fn value_parts(sh: &Ctx, pos: Pos, parts: &[Part]) -> R<Value> {
    if let Some(mut f) = single_func(parts) {
        return Ok(f.pop().unwrap());
    }
    if let Some((_, v)) = parts_single(parts) {
        return Ok(Value::str(v));
    }
    let guard = &mut Limit::guard(pos);
    let n_leaves = parts.iter().filter(|p| p.kind == Kind::Leaves).count();
    let n_list = parts.iter().filter(|p| p.kind == Kind::List).count();
    if n_leaves > 0 {
        let alts = parts_product(sh, parts, guard)?;
        if alts.len() > 1 {
            return Err(sh.error(pos, &format!("{} words in a variable definition", alts.len())));
        }
        return Ok(Value::string(alts.into_iter().next().unwrap_or_default()));
    }
    if n_list == 0 {
        return Ok(Value::string(parts.iter().map(|p| p.text()).collect()));
    }
    parts_list_value(sh, parts, n_list, guard)
}

/// The value of parts with n_list LazyLists of ${x}: with one, a LazyList
/// with the Strings before and after it on its leaves; with more, the
/// LazyList of the product of the leaves.
fn parts_list_value(sh: &Ctx, parts: &[Part], n_list: usize, limit: &mut Limit) -> R<Value> {
    if n_list == 1 {
        let mut left = String::new();
        for (i, p) in parts.iter().enumerate() {
            if p.kind == Kind::List {
                let right: String = parts[i + 1..].iter().map(|p| p.text()).collect();
                return value_affix(sh, &p.value, &left, &right, limit);
            }
            left.push_str(p.text());
        }
    }
    let alts = parts_product(sh, parts, limit)?;
    Ok(Value::List(LazyList::from_vec(alts.into_iter().map(Value::string).collect())))
}

/// The product of the options of the parts (the leaves of a LazyList).
fn parts_product(sh: &Ctx, parts: &[Part], limit: &mut Limit) -> R<Vec<String>> {
    let mut alts = vec![String::new()];
    for p in parts {
        let opts = if p.is_list() { value_leaves_in(sh, &p.value, limit)? } else { vec![p.text().to_string()] };
        let mut next = Vec::with_capacity(alts.len() * opts.len());
        for a in &alts {
            for o in &opts {
                next.push(format!("{}{}", a, o));
            }
        }
        alts = next;
    }
    Ok(alts)
}

/// The value with left before and right after each leaf (keeping the
/// nesting). Without them, the LazyList itself (still lazy). A function value
/// as a leaf is an error.
pub fn value_affix(sh: &Ctx, v: &Value, left: &str, right: &str, limit: &mut Limit) -> R<Value> {
    if left.is_empty() && right.is_empty() {
        return Ok(v.clone());
    }
    match v {
        Value::Str(s) => Ok(Value::string(format!("{}{}{}", left, s, right))),
        Value::Func(_) => Err(sh.error(limit.pos, FUNC_STRING)),
        Value::List(l) => {
            let items = l.to_vec()?;
            let mut out = Vec::with_capacity(items.len());
            for i in &items {
                out.push(value_affix(sh, i, left, right, limit)?);
            }
            Ok(Value::List(LazyList::from_vec(out)))
        }
    }
}

// ── Parts ──

/// The parts of the elements of a word. In a word of two or more elements,
/// a function value is an error.
pub fn parts(sh: &Ctx, elems: &[Sexp]) -> R<Vec<Part>> {
    let mut out = Vec::with_capacity(elems.len());
    for e in elems {
        let p = part(sh, e)?;
        if p.kind == Kind::Func && elems.len() > 1 {
            return Err(sh.error(e.pos(), FUNC_STRING));
        }
        out.push(p);
    }
    Ok(out)
}

fn part(sh: &Ctx, elem: &Sexp) -> R<Part> {
    let kind = elem.tag();
    let children = elem.children();
    let text = || children.first().map(|c| c.str()).unwrap_or("");
    match kind {
        "lit" => return Ok(Part::new(Kind::Plain, Value::str(text()))),
        "esc" | "sq" => return Ok(Part::new(Kind::Quoted, Value::str(text()))),
        "dq" => return Ok(Part::new(Kind::Quoted, Value::string(dq(sh, children)?))),
        _ => {}
    }
    let value = reference(sh, elem)?;
    if kind == "glob" {
        return Ok(Part { kind: Kind::List, value, active: false });
    }
    let is_var = kind == "var";
    Ok(match (&value, is_var) {
        (Value::Str(_), true) => Part::new(Kind::Plain, value),
        (Value::Str(_), false) => Part::new(Kind::Braced, value),
        (Value::List(_), true) => Part::new(Kind::Leaves, value),
        (Value::List(_), false) => Part::new(Kind::List, value),
        (Value::Func(_), _) => Part::new(Kind::Func, value),
    })
}

/// The value of a variable reference or a command substitution. ${x-value}
/// makes the value only if x is not defined.
fn reference(sh: &Ctx, elem: &Sexp) -> R<Value> {
    let kind = elem.tag();
    let pos = elem.pos();
    let children = elem.children();
    if kind == "subst" {
        return crate::eval::eval_subst(sh, pos, &children[0]);
    }
    if kind == "glob" {
        let parts = parts(sh, word_elems(&children[0]))?;
        let names = parts_fields_in(sh, &parts, &mut Limit::guard(pos), true)?;
        return Ok(Value::List(LazyList::from_vec(names)));
    }
    let name = children[0].str();
    if children.len() == 1 {
        return sh.lookup(pos, name);
    }
    if sh.defined(pos, name)? {
        return sh.lookup(pos, name);
    }
    def_value(sh, children[1].children())
}

/// The contents of "..." as one String. LazyList values are made Strings.
fn dq(sh: &Ctx, elems: &[Sexp]) -> R<String> {
    let mut acc = String::new();
    for e in elems {
        if e.tag() == "lit" {
            acc.push_str(e.children().first().map(|c| c.str()).unwrap_or(""));
        } else {
            let v = reference(sh, e)?;
            acc.push_str(&value_to_string(sh, e.pos(), &v)?);
        }
    }
    Ok(acc)
}

// ── Values ──

/// The leaves of a value. With a byte limit, each leaf is charged as it is
/// made, so that an endless LazyList stops when it is over the limit. A
/// function value is an error at the position of the limit.
pub fn value_leaves_in(sh: &Ctx, v: &Value, limit: &mut Limit) -> R<Vec<String>> {
    let mut out = Vec::new();
    leaves_into(sh, v, limit, &mut out)?;
    Ok(out)
}

fn leaves_into(sh: &Ctx, v: &Value, limit: &mut Limit, out: &mut Vec<String>) -> R<()> {
    match v {
        Value::Str(s) => {
            charge(sh, limit, s)?;
            out.push(s.to_string());
        }
        Value::Func(_) => return Err(sh.error(limit.pos, FUNC_STRING)),
        Value::List(l) => {
            let mut cur = l.clone();
            while let Some((h, t)) = cur.uncons()? {
                leaves_into(sh, &h, limit, out)?;
                cur = t;
            }
        }
    }
    Ok(())
}

pub fn values_leaves_in(sh: &Ctx, values: &[Value], limit: &mut Limit) -> R<Vec<String>> {
    let mut out = Vec::new();
    for v in values {
        leaves_into(sh, v, limit, &mut out)?;
    }
    Ok(out)
}

/// A value as one String: the leaves of a LazyList joined with OFS.
pub fn value_to_string(sh: &Ctx, pos: Pos, v: &Value) -> R<String> {
    match v {
        Value::Str(s) => Ok(s.to_string()),
        Value::Func(_) => Err(sh.error(pos, FUNC_STRING)),
        Value::List(_) => {
            let ofs = sh.ofs(pos)?;
            Ok(value_leaves_in(sh, v, &mut Limit::guard(pos))?.join(&ofs))
        }
    }
}

/// The command name and the other values of the LazyList of the first word.
/// Only the first element is expanded recursively: ((a (b c)) d) gives the
/// name a and the values (b c) and d.
pub fn value_head(sh: &Ctx, pos: Pos, v: &Value) -> R<(Value, Vec<Value>)> {
    let l = match v {
        Value::List(l) => l,
        v => return Ok((v.clone(), Vec::new())),
    };
    let (head, tail) = match l.uncons()? {
        None => return Err(sh.error(pos, "the command name is an empty LazyList")),
        Some(x) => x,
    };
    let rest = tail.to_vec()?;
    match head {
        Value::Str(_) | Value::Func(_) => Ok((head, rest)),
        h => {
            let (name, mut inner) = value_head(sh, pos, &h)?;
            inner.extend(rest);
            Ok((name, inner))
        }
    }
}

/// The command name, the other values from the first word, and the index of
/// the next word (Cmd_head). The first word that does not disappear gives
/// the name: a LazyList of ${x} or $( ) by value_head, otherwise as words of
/// an external command (charged to the limit). A word with a LazyList that
/// becomes empty is an error. None if every word disappears.
pub fn cmd_head(sh: &Ctx, words: &[Sexp], limit: &mut Limit) -> R<Option<(Value, Vec<Value>, usize)>> {
    for (i, word) in words.iter().enumerate() {
        let parts = parts(sh, word_elems(word))?;
        if let Some(mut f) = single_func(&parts) {
            return Ok(Some((f.pop().unwrap(), Vec::new(), i + 1)));
        }
        if let Some((kind, value)) = parts_single(&parts) {
            let fields = match single_fields(kind, value) {
                Some(f) => strings(f),
                None => parts_fields_in(sh, &parts, limit, false)?,
            };
            let mut it = fields.into_iter();
            match it.next() {
                None => continue,
                Some(name) => return Ok(Some((name, it.collect(), i + 1))),
            }
        }
        let n_leaves = parts.iter().filter(|p| p.kind == Kind::Leaves).count();
        let n_list = parts.iter().filter(|p| p.kind == Kind::List).count();
        if n_leaves == 0 && n_list > 0 {
            let value = parts_list_value(sh, &parts, n_list, limit)?;
            let (name, more) = value_head(sh, word.pos(), &value)?;
            return Ok(Some((name, more, i + 1)));
        }
        let fields = parts_fields_in(sh, &parts, limit, false)?;
        let mut it = fields.into_iter();
        match it.next() {
            None => {
                if n_leaves > 0 || n_list > 0 {
                    return Err(sh.error(word.pos(), "the command name is an empty LazyList"));
                }
            }
            Some(name) => return Ok(Some((name, it.collect(), i + 1))),
        }
    }
    Ok(None)
}
