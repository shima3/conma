//! Transpiled code of ConMa modules, made by the ConMa-to-Rust transpiler
//! vm/conma2rust/conma2rust.py (stage 2 of
//! item 7 of the ConMa review: running the parts of a program that need no
//! VProc switch as Rust, several steps at a time).
//!
//! A module of transpiled code belongs to one module file, identified by the
//! FNV-1a hash of its text. For each Function of the module (numbered in the
//! order in which load_module compiles them), it may have a transpiled function,
//! which stands for loading the Body of that Function: load_body calls it
//! instead. The transpiled function runs the Body and the Bodies after it as far
//! as it can without the VM (calls of primitives that only work on their
//! operands, passing values on with `__NULL__`, `__if_then_else__`, `__if__`
//! and `__unless__` with clauses written in place). Where a Body needs the VM
//! it is loaded (`deopt`), and `run` goes on doing the steps that follow as
//! the VM would, as long as they are applications of Closures with as many
//! operands as parameters (calls), of null (returns through the LCont or the
//! CChain), of escape continuations (`return` of __Cont_get__) and of the
//! primitives of SAFE_PRIMS, entering the transpiled code of the Bodies it
//! reaches; at anything else, or after BUDGET such steps, it leaves the VProc
//! to the VM. All this is one step of the VM. So the results are those of
//! the VM; only the number of steps (and so the order of the VProcs, and
//! traces) differ. With --trace or CONMA_PROFILE the transpiled code is not used.
//!
//! A call of a Closure of the module with an LCont written in place is a Rust
//! call (`call`, `rust_call`): the LCont stays in the Rust code instead of
//! going to the CChain as a Closure, and gets the values when the VProc
//! returns to the CChain the call started with. Leaving transpiled code in the
//! middle of Rust calls puts their CFrames into the CChain (Next::Leave), and
//! continuations made in them get those CFrames when the calls end (Frame).

#![allow(dead_code)] // without generated code, the helpers are unused

use crate::module::{make_closure, FuncDef, GVEnv};
use crate::prim::control::boolean_choice;
use crate::prim::{PResult, PrimDef, VmError};
use crate::sched::Machine;
use crate::step::{eval, load_body_interp};
use crate::value::*;
use crate::vproc::VProc;
use std::cell::Cell;
use std::rc::Rc;

/// What `run` does next.
pub enum Next {
    /// Run the Body of this Function in this LVEnv.
    Enter(usize, LVEnv),
    /// The VProc is as before a step of the VM: do that step if it is one
    /// that `apply` does.
    Apply,
    /// The values in the OList go back to the Rust call that is running
    /// (the VProc returned to the base of its Frame).
    Return,
    /// Leave the Rust calls: each puts its CFrame into the CChain, so that
    /// the VProc is as before a step of the VM with the CChain the VM would
    /// have.
    Leave,
    /// The Operator is a continuation made in a Rust call around this one:
    /// leave the Rust calls up to that one, which applies it.
    Escape,
}

pub type NResult = Result<Next, VmError>;

pub type TranspiledFn = fn(&mut Machine, &mut VProc, LVEnv, &Rc<GVEnv>, &Table) -> NResult;

/// The direct entry of a Function with a fused chain: its arguments as
/// Values (taken from the slice, one per parameter) instead of LVEnv nodes,
/// with the LVEnv of the Closure, and the LVEnv with the arguments if there
/// is one already (else the chain makes it when it needs it).
pub type DirectFn = fn(&mut Machine, &mut VProc, LVEnv, &mut [Value], Option<LVEnv>, &Rc<GVEnv>, &Table) -> NResult;

/// The arguments of a call made through a direct entry are moved into an
/// array of this many Values on the stack; a call with more is made with an
/// LVEnv.
const DIRECT_MAX: usize = 8;

/// The steps `run` does in one step of the VM at most, so that a VProc that
/// loops in transpiled code lets the others run.
const BUDGET: u32 = 1000;

/// Rust calls nested deeper than this are made as the VM makes them (the
/// LCont goes to the CChain), so that leaving them does not take long.
const MAX_DEPTH: usize = 256;

/// A call that transpiled code runs as a Rust call: its LCont is not in the
/// CChain but in the Rust code that called it.
pub struct Frame {
    id: u64,
    /// The CChain when it was called. The call returns when the VProc passes
    /// values to this CChain (null as the Operator without an LCont, or a
    /// continuation made in it whose CChain is this).
    base: Value,
    /// Continuations made while it was the innermost Rust call: their
    /// CChains lack its CFrame (and those of the Rust calls around it).
    conts: Vec<Rc<ContObj>>,
}

/// The Rust calls of transpiled code (Machine::nctx).
#[derive(Default)]
pub struct Ctx {
    frames: Vec<Frame>,
    ids: u64,
    budget: u32,
}

impl Ctx {
    fn current(&self) -> u64 {
        self.frames.last().map_or(0, |f| f.id)
    }
}

fn same_chain(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::LList(x), Value::LList(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

/// The CChain `chain`, which ends with `base`, with `node` (a CChain whose
/// tail is base) in place of base.
fn splice(chain: &Value, base: &Value, node: &Value) -> Value {
    let mut heads = Vec::new();
    let mut cur = chain;
    while !same_chain(cur, base) {
        match cur {
            Value::LList(cell) => {
                heads.push(cell.head.clone());
                cur = &cell.tail;
            }
            _ => {
                debug_assert!(false, "a CChain in a Rust call without its base");
                return chain.clone();
            }
        }
    }
    let mut out = node.clone();
    for h in heads.into_iter().rev() {
        out = Value::LList(Rc::new(LList { head: h, tail: out }));
    }
    out
}

/// Primitives `apply` calls: those of conma2rust.py's CHAIN (and __Ref_set__),
/// the conditionals and __Cont_get__. They only change the registers of the
/// VProc (and Refs, Dicts and so on); they do not block or switch VProcs.
const SAFE_PRIMS: &[&str] = &[
    "__is_null__", "__type_name__",
    "__LList_cons__", "__LList_uncons__", "__LList_append__", "__LList_length__",
    "__Number_fromString__", "__Number_toString__", "__Number_is_zero__",
    "__Number_is_positive__", "__Number_is_negative__", "__Number_add__",
    "__Number_subtract__", "__Number_multiply__", "__Number_divide__", "__Number_floor__", "__Number_sqrt__",
    "__String_find_any__", "__String_strip_suffix__", "__String_length__",
    "__String_substring__", "__String_find__", "__Sexp_read__", "__String_concat__",
    "__String_toCode__", "__String_fromCode__", "__String_equal__",
    "__Dict_new__", "__Dict_put__", "__Dict_lookup_or__", "__Dict_remove__",
    "__Dict_keys__", "__Dict_copy__",
    "__MSeq_new__", "__MSeq_is_empty__", "__MSeq_uncons__", "__MSeq_set_head__", "__MSeq_append__",
    "__Ref_new__", "__Ref_get__", "__PDict_put__", "__PDict_lookup_or__",
    "__if_then_else__", "__if__", "__unless__", "__Cont_get__",
];

/// The transpiled code of one module file, as the generated code gives it.
pub struct Module {
    pub hash: u64,
    /// The number of Functions in the module.
    pub nfuncs: usize,
    /// The primitives the transpiled code calls, by ConMa name; `prim_call`
    /// takes an index into this list.
    pub prims: &'static [&'static str],
    pub fns: &'static [Option<TranspiledFn>],
    pub dfns: &'static [Option<DirectFn>],
}

/// The transpiled code of a loaded module, with the Functions it refers to.
pub struct Table {
    pub funcs: Vec<Rc<FuncDef>>,
    pub fns: &'static [Option<TranspiledFn>],
    pub dfns: &'static [Option<DirectFn>],
    pub prims: Vec<&'static PrimDef>,
    /// The PrimDefs of SAFE_PRIMS.
    safe: Vec<*const PrimDef>,
    /// For each Function, how often a Rust call of it returned values and
    /// how often it was left (to the VM, or by an escape): see rust_ok.
    rust_calls: Vec<(Cell<u32>, Cell<u32>)>,
}

impl Table {
    /// Whether to call Function fid as a Rust call. A Rust call that is left
    /// costs more than a call made as the VM makes it (its LCont, LVEnv and
    /// CFrame are made then after all, and the CChain is spliced), so a
    /// Function whose Rust calls are left more than 8 times, and more than a
    /// third as often as they return values, is called as the VM calls it
    /// from then on (ConSh, whose commands reach a step of the VM through many
    /// calls; the Functions of nbody are never left).
    #[inline]
    pub fn rust_ok(&self, fid: usize) -> bool {
        let (values, left) = &self.rust_calls[fid];
        let left = left.get();
        left <= 8 || left * 3 <= values.get()
    }

    fn note(&self, fid: usize, r: &Result<Ret, VmError>) {
        let (values, left) = &self.rust_calls[fid];
        match r {
            Ok(Ret::Values) => values.set(values.get().saturating_add(1)),
            _ => left.set(left.get().saturating_add(1)),
        }
    }
}

mod generated {
    #![allow(unused_variables, unused_mut)]
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/transpiled_gen.rs"));
}

/// FNV-1a (64 bits) of a module file, as vm/conma2rust/conma2rust.py computes it.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// The Table for the module file with this hash, if there is transpiled code for
/// it and it has the same number of Functions.
pub fn table_for(hash: u64, funcs: Vec<Rc<FuncDef>>) -> Option<Rc<Table>> {
    let m = generated::MODULES.iter().find(|m| m.hash == hash)?;
    if m.nfuncs != funcs.len() || m.fns.len() != funcs.len() || m.dfns.len() != funcs.len() {
        return None;
    }
    let mut prims = Vec::new();
    for name in m.prims {
        prims.push(crate::prim::lookup(name)?);
    }
    let mut safe: Vec<*const PrimDef> = SAFE_PRIMS.iter().filter_map(|n| crate::prim::lookup(n)).map(|p| p as *const PrimDef).collect();
    safe.sort();
    let rust_calls = (0..funcs.len()).map(|_| (Cell::new(0), Cell::new(0))).collect();
    Some(Rc::new(Table { funcs, fns: m.fns, dfns: m.dfns, prims, safe, rust_calls }))
}

// ── The loop ──

/// Run the Body of Function fid in env (load_body calls this instead of
/// loading it), and the steps after it that `apply` does, up to BUDGET.
pub fn run(m: &mut Machine, vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {
    debug_assert!(m.nctx.frames.is_empty());
    BOOLS.with(|b| {
        if b.borrow().is_none() {
            *b.borrow_mut() = Some((m.false_closure(), m.true_closure()));
        }
    });
    m.nctx.budget = BUDGET;
    drive(m, vp, Next::Enter(fid, env), gv, t)?;
    Ok(())
}

thread_local! {
    /// The Booleans of the Machine (false, true), for bool_value.
    static BOOLS: std::cell::RefCell<Option<(Value, Value)>> = const { std::cell::RefCell::new(None) };
}

/// The Boolean of a bool that a fused chain keeps in a Rust variable (the
/// result of `__is_null__` or a Number predicate), where it is used as a
/// Value. Without a Machine at hand (in a CFrame builder), from BOOLS, which
/// `run` sets before any transpiled code runs.
pub fn bool_value(b: bool) -> Value {
    BOOLS.with(|v| {
        let v = v.borrow();
        let (f, t) = v.as_ref().expect("the Booleans of the Machine");
        if b { t.clone() } else { f.clone() }
    })
}

/// What `drive` ends with.
enum Ret {
    /// Values in the OList, for the Rust call (Next::Return).
    Values,
    /// The VM does the next step (or, in a Rust call, it is left).
    Leave,
    /// Next::Escape.
    Escape,
}

/// Do `next` and the steps after it, in the innermost Rust call (or with
/// none, for `run`).
fn drive(m: &mut Machine, vp: &mut VProc, mut next: Next, gv: &Rc<GVEnv>, t: &Table) -> Result<Ret, VmError> {
    let in_frame = !m.nctx.frames.is_empty();
    loop {
        next = match next {
            Next::Enter(fid, env) => match t.fns[fid] {
                Some(f) => f(m, vp, env, gv, t)?,
                None => deopt(vp, fid, env, gv, t)?,
            },
            Next::Apply => {
                if m.nctx.budget == 0 {
                    return Ok(Ret::Leave);
                }
                m.nctx.budget -= 1;
                match apply(m, vp, gv, t)? {
                    Some(n) => n,
                    None => return Ok(Ret::Leave),
                }
            }
            Next::Return => return Ok(Ret::Values),
            // Out of all Rust calls: go on as the VM would.
            Next::Leave if !in_frame => Next::Apply,
            Next::Leave => return Ok(Ret::Leave),
            Next::Escape => match &vp.operator {
                Value::Cont(k) if in_frame && k.owner.get() != m.nctx.current() => return Ok(Ret::Escape),
                _ => Next::Apply,
            },
        };
    }
}

/// Do the next step of the VM if it is one of these, as `step` would do it:
/// null as the Operator (pass the OList to the LCont, or to the top CFrame of
/// the CChain), a Closure of this module with as many operands as its
/// parameters (bind them; the LCont goes to the CChain), an escape
/// continuation (set the CChain back), a primitive of SAFE_PRIMS. None for
/// anything else: the VM does that step. In a Rust call, values passed to
/// the CChain it was called with go back to it (Next::Return).
fn apply(m: &mut Machine, vp: &mut VProc, gv: &Rc<GVEnv>, t: &Table) -> Result<Option<Next>, VmError> {
    match &vp.operator {
        Value::Null => {
            if let Some(lc) = vp.lcont.take() {
                vp.operator = lc;
                return Ok(Some(Next::Apply));
            }
            if let Some(f) = m.nctx.frames.last() {
                if same_chain(&vp.cchain, &f.base) {
                    return Ok(Some(Next::Return));
                }
            }
            match vp.cchain_pop() {
                Some(cf) => {
                    vp.operator = cf.closure.clone();
                    vp.sinfo = cf.sinfo.clone();
                    Ok(Some(Next::Apply))
                }
                None => Ok(None),
            }
        }
        Value::Closure(c) => {
            if c.variadic || c.params().len() != vp.olist.len() {
                return Ok(None);
            }
            if c.body.as_ref().is_some_and(|b| b.func_id.is_none()) {
                // A Boolean (`,(t f) __NULL__ t` or `f`): pass on the operand
                // it chooses.
                if let Some(b) = boolean_choice(&vp.operator) {
                    let v = vp.olist.swap_remove(if b { 0 } else { 1 });
                    vp.olist.clear();
                    vp.olist.push(v);
                    vp.operator = Value::Null;
                    return Ok(Some(Next::Apply));
                }
                return Ok(None);
            }
            let same_module = match &c.gvenv {
                Some(g) => Rc::ptr_eq(g, gv),
                None => Rc::ptr_eq(&vp.gvenv, gv),
            };
            let Some(fid) = c.body.as_ref().and_then(|b| b.func_id) else { return Ok(None) };
            if !same_module {
                return Ok(None);
            }
            let c = c.clone();
            if let Some(lc) = vp.lcont.take() {
                let sinfo = vp.sinfo.clone();
                vp.cchain_push(lc, sinfo);
            }
            let n = vp.olist.len();
            if let Some(d) = t.dfns[fid].filter(|_| c.skip == 0 && n <= DIRECT_MAX) {
                // The arguments go to the direct entry, not into an LVEnv.
                let mut a: [Value; DIRECT_MAX] = std::array::from_fn(|_| Value::Null);
                for (k, v) in vp.olist.drain(..).enumerate() {
                    a[k] = v;
                }
                vp.operator = Value::Null;
                return d(m, vp, c.lvenv.clone(), &mut a[..n], None, gv, t).map(Some);
            }
            let params = c.params();
            let mut env = c.lvenv.clone();
            for (k, v) in vp.olist.drain(..).enumerate() {
                env = lv_push(&env, params[k].clone(), v);
            }
            vp.operator = Value::Null;
            Ok(Some(Next::Enter(fid, env)))
        }
        Value::Cont(k) => {
            // A continuation made in another Rust call (or out of them, or
            // in one that has ended): leave the Rust calls, which gives it
            // its whole CChain, and apply it as the VM would.
            if k.owner.get() != m.nctx.current() && !m.nctx.frames.is_empty() {
                // Made out of the Rust calls (owner 0) or after one has ended:
                // they all must be left. Made in a Rust call around this one:
                // that one applies it.
                let owner = k.owner.get();
                if owner != 0 && m.nctx.frames.iter().any(|f| f.id == owner) {
                    return Ok(Some(Next::Escape));
                }
                return Ok(Some(Next::Leave));
            }
            let kc = k.cchain.borrow().clone();
            vp.lcont = None;
            vp.operator = Value::Null;
            if kc.is_null() && !m.nctx.frames.last().is_some_and(|f| f.base.is_null()) {
                vp.olist.clear(); // as the VM: the VProc ends
            }
            vp.cchain = kc;
            // Now null as the Operator: return the values to the top CFrame
            // (or to the Rust call).
            Ok(Some(Next::Apply))
        }
        Value::Prim(p) => {
            let p: &'static PrimDef = *p;
            if t.safe.binary_search(&(p as *const PrimDef)).is_err() {
                return Ok(None);
            }
            if p.name == "__Cont_get__" {
                // Without an LCont, the LCont is the top CFrame: in a Rust
                // call that may be the Rust caller's, which only the VM sees.
                let Some(lc) = vp.lcont.take() else {
                    return Ok(if m.nctx.frames.is_empty() { (p.f)(m, vp).map(|_| Some(Next::Apply))? } else { Some(Next::Leave) });
                };
                let k = make_cont(m, vp);
                vp.operator = lc;
                vp.olist.clear();
                vp.olist.push(k);
                return Ok(Some(Next::Apply));
            }
            (p.f)(m, vp)?;
            Ok(Some(Next::Apply))
        }
        _ => Ok(None),
    }
}

/// A new escape continuation with the CChain of the VProc, recorded in the
/// innermost Rust call.
pub fn make_cont(m: &mut Machine, vp: &mut VProc) -> Value {
    m.serials.conts += 1;
    let k = Rc::new(ContObj::new(m.serials.conts, vp.cchain.clone()));
    if let Some(f) = m.nctx.frames.last_mut() {
        k.owner.set(f.id);
        f.conts.push(k.clone());
    }
    Value::Cont(k)
}

/// A Rust call: run Function fid in env as a call with the LCont Function lc
/// (in lenv, with sinfo for its CFrame), then bind the values it returns to
/// lc.
#[allow(clippy::too_many_arguments)]
fn rust_call(m: &mut Machine, vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: usize, lenv: LVEnv, sinfo: Option<SInfo>) -> NResult {
    if m.nctx.frames.len() >= MAX_DEPTH {
        let lcv = make_closure(&t.funcs[lc], &lenv, Some(gv));
        vp.cchain_push(lcv, sinfo);
        return Ok(Next::Enter(fid, env));
    }
    let mut env = Some(env);
    let first = &mut |_: &mut Machine, _: &mut VProc| Ok(Next::Enter(fid, env.take().expect("entered once")));
    match frame_run(m, vp, fid, first, gv, t, &|b: &Value| cframe_on(t, gv, lc, lenv.clone(), &sinfo, b.clone()), &sinfo)? {
        Ret::Values => {
            vp.lcont = None;
            pass_results(vp, lenv, gv, t, Some(lc))
        }
        Ret::Leave => Ok(Next::Leave),
        Ret::Escape => Ok(Next::Escape),
    }
}

/// The CChain base with the CFrame of the LCont Function lc in lenv (with
/// sinfo) on top, as the VM pushes it for a call.
pub fn cframe_on(t: &Table, gv: &Rc<GVEnv>, lc: usize, lenv: LVEnv, sinfo: &Option<SInfo>, base: Value) -> Value {
    let cf = Value::CFrame(Rc::new(CFrame { closure: make_closure(&t.funcs[lc], &lenv, Some(gv)), sinfo: sinfo.clone() }));
    Value::LList(Rc::new(LList { head: cf, tail: base }))
}

/// Run Function fid in env as a Rust call. node puts on a CChain the CFrames
/// the VM would have pushed for the call (its LCont, and in a fused chain the
/// LConts of the clauses it is in), and is called only if they are needed: to
/// put them into the CChain when the call is left, or to give them to
/// continuations made in the call that are still in use. With Values, the
/// values are in the OList and the SInfo is sinfo (of the calling Body), as
/// after the VM pops the CFrame.
#[allow(clippy::too_many_arguments)]
fn frame_run(m: &mut Machine, vp: &mut VProc, fid: usize, first: &mut dyn FnMut(&mut Machine, &mut VProc) -> NResult, gv: &Rc<GVEnv>, t: &Table, node: &dyn Fn(&Value) -> Value, sinfo: &Option<SInfo>) -> Result<Ret, VmError> {
    m.nctx.ids += 1;
    let id = m.nctx.ids;
    m.nctx.frames.push(Frame { id, base: vp.cchain.clone(), conts: Vec::new() });
    // The first step (entering the Body of fid), then the steps after it
    // (none if the chain returned its values to this call at once).
    let r = match first(m, vp) {
        Ok(Next::Return) => Ok(Ret::Values),
        Ok(next) => drive(m, vp, next, gv, t),
        Err(e) => Err(e),
    };
    let f = m.nctx.frames.pop().expect("the Frame of a Rust call");
    t.note(fid, &r);
    if f.conts.is_empty() && matches!(r, Ok(Ret::Values)) {
        // The usual end: values, and no continuation to give the CFrame.
        vp.lvenv = None;
        vp.operator = Value::Null;
        vp.sinfo = sinfo.clone();
        return Ok(Ret::Values);
    }
    // The CFrame the VM would have pushed, made only when it is needed.
    let mut made: Option<Value> = None;
    let base = f.base.clone();
    let node_of = |n: &mut Option<Value>| -> Value { n.get_or_insert_with(|| node(&base)).clone() };
    if matches!(r, Ok(Ret::Values)) {
        // Drop what still holds the continuations made in the call.
        vp.lvenv = None;
        vp.operator = Value::Null;
    }
    // Left: the VM goes on with the CChain the VM would have. (Escape: the
    // continuation will replace the CChain.)
    if matches!(r, Ok(Ret::Leave)) {
        let n = node_of(&mut made);
        vp.cchain = splice(&vp.cchain, &f.base, &n);
    }
    // Continuations still in use get the CFrame, and belong to the Rust call
    // around this one.
    let parent = m.nctx.current();
    for k in f.conts {
        if Rc::strong_count(&k) > 1 {
            let n = node_of(&mut made);
            let c = splice(&k.cchain.borrow(), &f.base, &n);
            *k.cchain.borrow_mut() = c;
            k.owner.set(parent);
            if let Some(p) = m.nctx.frames.last_mut() {
                p.conts.push(k);
            }
        }
    }
    let r = r?;
    if matches!(r, Ret::Values) {
        // As the VM pops the CFrame: the SInfo of the calling Body.
        vp.sinfo = sinfo.clone();
    }
    Ok(r)
}

/// What a call from a fused chain gives.
pub enum Called {
    /// The callee returned its values, which are in the OList (which keeps
    /// its buffer): the chain binds them to its LCont.
    Values,
    /// The chain ends: go on with this.
    Next(Next),
}

/// A call `f a ... ,(x ...) ...` in a fused chain. A Closure of the module
/// with transpiled code and as many operands as parameters is run as a Rust
/// call; the chain gets the values it returns. Anything else is applied as the
/// VM would, with the LCont Closure (lc in the LVEnv that lenv makes).
#[allow(clippy::too_many_arguments)]
pub fn fused_call<const N: usize>(m: &mut Machine, vp: &mut VProc, f: Value, args: [Value; N], gv: &Rc<GVEnv>, t: &Table, node: &dyn Fn(&Value) -> Value, sinfo: &Option<SInfo>) -> Result<Called, VmError> {
    vp.sinfo = sinfo.clone();
    let target = match &f {
        Value::Closure(c) if !c.variadic && c.params().len() == N => {
            let same_module = match &c.gvenv {
                Some(g) => Rc::ptr_eq(g, gv),
                None => Rc::ptr_eq(&vp.gvenv, gv),
            };
            match c.body.as_ref().and_then(|b| b.func_id) {
                Some(cf) if same_module => Some((c, cf)),
                _ => None,
            }
        }
        _ => None,
    };
    let rust = m.nctx.budget > 0 && m.nctx.frames.len() < MAX_DEPTH && target.as_ref().is_some_and(|(_, cf)| t.rust_ok(*cf));
    let Some((c, cf)) = target.filter(|_| rust) else {
        // As the VM applies f with the LCont: its CFrame (and those of the
        // clauses around the call) on the CChain.
        vp.cchain = node(&vp.cchain);
        vp.operator = f;
        vp.olist.clear();
        vp.olist.extend(args);
        vp.lcont = None;
        return Ok(Called::Next(Next::Apply));
    };
    m.nctx.budget -= 1;
    vp.operator = Value::Null;
    vp.lcont = None;
    let r = match t.dfns[cf].filter(|_| c.skip == 0) {
        // The arguments go to the direct entry, not into an LVEnv.
        Some(d) => {
            let mut args = args;
            let base = c.lvenv.clone();
            let mut base = Some(base);
            let first = &mut |m: &mut Machine, vp: &mut VProc| d(m, vp, base.take().expect("entered once"), &mut args, None, gv, t);
            frame_run(m, vp, cf, first, gv, t, node, sinfo)?
        }
        None => {
            let params = c.params();
            let mut e = c.lvenv.clone();
            for (k, v) in args.into_iter().enumerate() {
                e = lv_push(&e, params[k].clone(), v);
            }
            let mut e = Some(e);
            let first = &mut |_: &mut Machine, _: &mut VProc| Ok(Next::Enter(cf, e.take().expect("entered once")));
            frame_run(m, vp, cf, first, gv, t, node, sinfo)?
        }
    };
    Ok(match r {
        Ret::Values => Called::Values,
        Ret::Leave => Called::Next(Next::Leave),
        Ret::Escape => Called::Next(Next::Escape),
    })
}

/// Values (in the OList) a fused chain cannot bind to its LCont lc (another
/// number of them, or a variadic lc): apply the LCont Closure to them as the
/// VM would.
pub fn fused_pass(vp: &mut VProc, gv: &Rc<GVEnv>, t: &Table, lc: usize, lenv: LVEnv) -> NResult {
    vp.operator = make_closure(&t.funcs[lc], &lenv, Some(gv));
    vp.lcont = None;
    Ok(Next::Apply)
}

// ── Helpers called by the generated code ──

#[derive(Clone, Copy)]
pub enum Cond {
    /// The Boolean is the value of the first operand.
    Value,
    /// The first operand is `(,() prim a ...)` (Function number, index of the
    /// primitive), a primitive that only reads its operands and returns one
    /// value: it is called here instead of making the thunk.
    Thunk(usize, usize),
}

#[derive(Clone, Copy)]
pub enum Clause {
    /// The clause is the operand at this index; only __nop__ is run here.
    Value(usize),
    /// The clause is `(,() ...)`, the Function with this number.
    Thunk(usize),
    /// __nop__ (the missing clause of __if__ and __unless__).
    Nop,
}

fn body(t: &Table, fid: usize) -> &crate::module::BodyDef {
    t.funcs[fid].body.as_ref().expect("a transpiled Function has a Body")
}

/// Load the Body of Function fid for the VM, without its transpiled code; `run`
/// goes on with the step after it.
pub fn deopt(vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> NResult {
    load_body_interp(vp, body(t, fid), env, gv)?;
    Ok(Next::Apply)
}

/// The operands of the Body of fid, evaluated in env, as the OList.
fn load_operands(vp: &mut VProc, fid: usize, env: &LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {
    let mut olist = std::mem::take(&mut vp.olist);
    olist.clear();
    for c in &body(t, fid).olist {
        match eval(c, env, gv) {
            Ok(v) => olist.push(v),
            Err(e) => {
                olist.clear();
                vp.olist = olist;
                return Err(e);
            }
        }
    }
    vp.olist = olist;
    Ok(())
}

/// The VProc is as after a primitive returned its results (the OList) with
/// no LCont: pass them to the LCont Function lc of the Body, as the VM would
/// apply the LCont Closure to them. With the same number of values as its
/// parameters (and not variadic), bind them and run its Body; otherwise make
/// the Closure and go on as the VM would.
fn pass_results(vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: Option<usize>) -> NResult {
    let Some(l) = lc else {
        vp.lvenv = env; // no LCont: the values go to the CChain
        return Ok(Next::Apply);
    };
    let f = &t.funcs[l];
    if f.variadic || vp.olist.len() != f.params.len() {
        vp.operator = make_closure(f, &env, Some(gv));
        vp.lvenv = env;
        return Ok(Next::Apply);
    }
    let mut e = env;
    for (k, v) in vp.olist.drain(..).enumerate() {
        e = lv_push(&e, f.params[k].clone(), v);
    }
    Ok(Next::Enter(l, e))
}

/// After a primitive called with the LCont set to a mark (Null): if it
/// returned its results, pass them on; otherwise (onError, a Closure to
/// apply) put the real LCont back and go on as the VM would.
fn after_prim(vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: Option<usize>) -> NResult {
    if vp.operator.is_null() && vp.lcont.is_none() {
        return pass_results(vp, env, gv, t, lc);
    }
    if matches!(vp.lcont, Some(Value::Null)) {
        vp.lcont = lc.map(|l| make_closure(&t.funcs[l], &env, Some(gv)));
    }
    vp.lvenv = env;
    Ok(Next::Apply)
}

/// `prim a ... [,(x ...) ...]`: the Body of fid calls primitive p (an index
/// into Table::prims) with the LCont Function lc.
#[allow(clippy::too_many_arguments)]
pub fn prim_call(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, p: usize, lc: Option<usize>) -> NResult {
    load_operands(vp, fid, &env, gv, t)?;
    let prim = t.prims[p];
    vp.sinfo = body(t, fid).sinfo.clone();
    vp.operator = Value::Prim(prim);
    vp.lcont = Some(Value::Null);
    (prim.f)(m, vp)?;
    after_prim(vp, env, gv, t, lc)
}

/// `__NULL__ a ... [,(x ...) ...]`: pass the operands to the LCont Function
/// lc, or with none, return them.
pub fn null_pass(vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, lc: Option<usize>) -> NResult {
    load_operands(vp, fid, &env, gv, t)?;
    vp.sinfo = body(t, fid).sinfo.clone();
    vp.operator = Value::Null;
    vp.lcont = None;
    pass_results(vp, env, gv, t, lc)
}

/// `f a ... [,(x ...) ...]` with f a variable: a call of a Closure of this
/// module with transpiled code and as many operands as its parameters is run in
/// place, as a Rust call if there is an LCont; anything else is left to the
/// VM's way (deopt).
pub fn call(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, lc: Option<usize>) -> NResult {
    let b = body(t, fid);
    let f = eval(b.operator.as_ref().expect("an Operator"), &env, gv)?;
    let target = match &f {
        Value::Closure(c) if !c.variadic && c.params().len() == b.olist.len() => {
            let same_module = match &c.gvenv {
                Some(g) => Rc::ptr_eq(g, gv),
                None => Rc::ptr_eq(&vp.gvenv, gv),
            };
            match c.body.as_ref().and_then(|b| b.func_id) {
                Some(cf) if same_module => Some((c, cf)),
                _ => None,
            }
        }
        _ => None,
    };
    // A call is a step of `apply` for the BUDGET (a loop of tail calls must
    // let the other VProcs run).
    let Some((c, cf)) = target.filter(|_| m.nctx.budget > 0) else { return deopt(vp, fid, env, gv, t) };
    m.nctx.budget -= 1;
    let params = c.params();
    let mut e = c.lvenv.clone();
    for (k, o) in b.olist.iter().enumerate() {
        e = lv_push(&e, params[k].clone(), eval(o, &env, gv)?);
    }
    vp.sinfo = b.sinfo.clone();
    vp.operator = Value::Null;
    vp.lcont = None;
    match lc {
        Some(l) if t.rust_ok(cf) => rust_call(m, vp, cf, e, gv, t, l, env, b.sinfo.clone()),
        Some(l) => {
            let lcv = make_closure(&t.funcs[l], &env, Some(gv));
            vp.cchain_push(lcv, b.sinfo.clone());
            Ok(Next::Enter(cf, e))
        }
        None => Ok(Next::Enter(cf, e)),
    }
}

/// `__if_then_else__`, `__if__` or `__unless__` in the Body of fid, with the
/// LCont Function lc. Falls back to the VM (deopt) when the Boolean is not
/// one of the two Boolean Closures or the clause is a value other than
/// __nop__.
#[allow(clippy::too_many_arguments)]
pub fn if_call(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, cond: Cond, then: Clause, els: Clause, lc: Option<usize>) -> NResult {
    let b = body(t, fid);
    let boolean = match cond {
        Cond::Value => eval(&b.olist[0], &env, gv)?,
        Cond::Thunk(cf, p) => {
            load_operands(vp, cf, &env, gv, t)?;
            let prim = t.prims[p];
            vp.sinfo = body(t, cf).sinfo.clone();
            vp.operator = Value::Prim(prim);
            vp.lcont = Some(Value::Null);
            (prim.f)(m, vp)?;
            if vp.operator.is_null() && vp.lcont.is_none() && vp.olist.len() == 1 {
                vp.olist.pop().unwrap()
            } else {
                return deopt(vp, fid, env, gv, t);
            }
        }
    };
    let Some(choice) = boolean_choice(&boolean) else {
        return deopt(vp, fid, env, gv, t);
    };
    let clause = if choice { then } else { els };
    let nop = match clause {
        Clause::Nop => true,
        Clause::Value(i) => {
            let v = eval(&b.olist[i], &env, gv)?;
            if !m.is_nop(&v) {
                return deopt(vp, fid, env, gv, t);
            }
            true
        }
        Clause::Thunk(_) => false,
    };
    vp.olist.clear();
    vp.sinfo = b.sinfo.clone();
    if nop {
        // __nop__ returns nothing to the LCont.
        vp.operator = Value::Null;
        vp.lcont = None;
        return pass_results(vp, env, gv, t, lc);
    }
    let Clause::Thunk(cf) = clause else { unreachable!() };
    // The thunk is applied with the LCont: a Rust call.
    vp.lcont = None;
    vp.operator = Value::Null;
    match lc {
        Some(l) => {
            let sinfo = b.sinfo.clone();
            if t.rust_ok(cf) {
                rust_call(m, vp, cf, env.clone(), gv, t, l, env, sinfo)
            } else {
                let lcv = make_closure(&t.funcs[l], &env, Some(gv));
                vp.cchain_push(lcv, sinfo);
                Ok(Next::Enter(cf, env))
            }
        }
        None => Ok(Next::Enter(cf, env)),
    }
}

// ── Direct calls of primitives (the generated code evaluates the operands
// itself and calls these, without the OList; conma2rust.py's DIRECT) ──

/// What a primitive called directly gives.
pub enum Out {
    R0,
    R1(Value),
    R2(Value, Value),
    /// Apply this (onError, onMissing) to one value, with the LCont.
    Call(Value, Value),
}

/// The Body of Function fid (for the generated code).
pub fn body_of(t: &Table, fid: usize) -> &crate::module::BodyDef {
    body(t, fid)
}

/// Go on after a direct call in a Body with the LCont Function lc, as the VM
/// would after the primitive returned (or applied onError).
pub fn finish(vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: Option<usize>, out: Out) -> NResult {
    let (n, a, b) = match out {
        Out::Call(f, x) => {
            vp.operator = f;
            vp.olist.clear();
            vp.olist.push(x);
            vp.lcont = lc.map(|l| make_closure(&t.funcs[l], &env, Some(gv)));
            vp.lvenv = env;
            return Ok(Next::Apply);
        }
        Out::R0 => (0, None, None),
        Out::R1(x) => (1, Some(x), None),
        Out::R2(x, y) => (2, Some(x), Some(y)),
    };
    if let Some(l) = lc {
        let f = &t.funcs[l];
        if !f.variadic && f.params.len() == n {
            let mut e = env;
            for (k, v) in a.into_iter().chain(b).enumerate() {
                e = lv_push(&e, f.params[k].clone(), v);
            }
            return Ok(Next::Enter(l, e));
        }
    }
    vp.olist.clear();
    vp.olist.extend(a.into_iter().chain(b));
    vp.operator = Value::Null;
    vp.lcont = None;
    pass_results(vp, env, gv, t, lc)
}

use crate::prim::{basic, collections, llist, number, string};

pub fn d_is_null(m: &mut Machine, a: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(basic::is_null_core(m, a)))
}
pub fn d_type_name(_m: &mut Machine, a: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(basic::type_name_core(a)?))
}
pub fn d_number_add(_m: &mut Machine, a: &Value, b: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_add__", a, b, |x, y| x + y)?))
}
pub fn d_number_subtract(_m: &mut Machine, a: &Value, b: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_subtract__", a, b, |x, y| x - y)?))
}
pub fn d_number_multiply(_m: &mut Machine, a: &Value, b: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_multiply__", a, b, |x, y| x * y)?))
}
pub fn d_number_is_zero(m: &mut Machine, a: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_zero__", a, |x| x == 0.0)?))
}
pub fn d_number_is_positive(m: &mut Machine, a: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_positive__", a, |x| x > 0.0)?))
}
pub fn d_number_is_negative(m: &mut Machine, a: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_negative__", a, |x| x < 0.0)?))
}
pub fn d_string_equal(m: &mut Machine, a: &Value, b: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(string::equal_core(m, a, b)?))
}
pub fn d_string_concat(_m: &mut Machine, a: &Value, b: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(string::concat_core(a, b)?))
}
pub fn d_llist_cons(_m: &mut Machine, head: &Value, tail: &Value) -> Result<Out, VmError> {
    Ok(Out::R1(Value::LList(Rc::new(LList { head: head.clone(), tail: tail.clone() }))))
}
pub fn d_llist_uncons(_m: &mut Machine, on_error: &Value, lst: &Value) -> Result<Out, VmError> {
    Ok(match llist::uncons_core(lst) {
        Ok((h, t)) => Out::R2(h, t),
        Err(msg) => Out::Call(on_error.clone(), string_value(&msg)),
    })
}
pub fn d_dict_lookup_or(_m: &mut Machine, d: &Value, key: &Value, on_missing: &Value) -> Result<Out, VmError> {
    Ok(match collections::dict_lookup_core(d, key)? {
        Some(v) => Out::R1(v),
        None => Out::Call(on_missing.clone(), key.clone()),
    })
}
pub fn d_dict_put(_m: &mut Machine, d: &Value, key: &Value, value: &Value) -> Result<Out, VmError> {
    collections::dict_put_core(d, key.clone(), value.clone())?;
    Ok(Out::R0)
}

/// `__Cont_get__ ,(return) ...`: the escape continuation of the calling
/// function, passed to the LCont written in place.
pub fn cont_get(m: &mut Machine, vp: &mut VProc) -> Result<Out, VmError> {
    Ok(Out::R1(make_cont(m, vp)))
}

pub fn d_number_divide(_m: &mut Machine, on_error: &Value, a: &Value, b: &Value) -> Result<Out, VmError> {
    let x = number::check_number("__Number_divide__", a)?;
    let y = number::check_number("__Number_divide__", b)?;
    if y == 0.0 {
        return Ok(Out::Call(on_error.clone(), string_value("__Number_divide__: division by zero")));
    }
    Ok(Out::R1(num(x / y)))
}
pub fn d_number_sqrt(_m: &mut Machine, on_error: &Value, a: &Value) -> Result<Out, VmError> {
    let x = number::check_number("__Number_sqrt__", a)?;
    if x < 0.0 {
        return Ok(Out::Call(on_error.clone(), string_value("__Number_sqrt__: square root of a negative number")));
    }
    Ok(Out::R1(num(x.sqrt())))
}

// ── Fused chains (conma2rust.py, Fuser): one Rust function runs a Body and
// the Bodies after it with the variables they bind in Rust variables ──

/// The constant operand k (a String or null) of the Body of Function fid.
pub fn cref(t: &Table, fid: usize, k: usize) -> &Value {
    match &body(t, fid).olist[k] {
        crate::module::CExpr::C(v) => v,
        _ => unreachable!("cref of an operand that is not a constant"),
    }
}

/// The local variable i of the LVEnv the fused chain started with.
pub fn lget(env: &LVEnv, i: usize) -> Result<Value, VmError> {
    match lv_get(env, i) {
        Some(v) => Ok(v.clone()),
        None => crate::prim::err(format!("LVEnv[{}] out of range", i)),
    }
}

/// The LVEnv env without its first n nodes (the parameters of the Function
/// it was made for): the LVEnv of the Closure.
pub fn lv_skip(env: &LVEnv, n: usize) -> LVEnv {
    let mut e = env;
    for _ in 0..n {
        match e {
            Some(node) => e = &node.next,
            None => break,
        }
    }
    e.clone()
}

/// Whether the call of f without an LCont, with n operands, at the end of a
/// fused chain that loops (Function fid with n parameters, run in a Closure
/// with the LVEnv base) calls that same Closure again: the chain then binds
/// the new arguments and starts again. The call counts against the budget
/// as `apply` would count it, so that a loop lets the other VProcs run.
#[inline]
pub fn self_call(m: &mut Machine, vp: &VProc, f: &Value, fid: usize, n: usize, base: &LVEnv, gv: &Rc<GVEnv>) -> bool {
    if m.nctx.budget == 0 {
        return false;
    }
    let Value::Closure(c) = f else { return false };
    let same_env = match (&c.lvenv, base) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    };
    let same_module = match &c.gvenv {
        Some(g) => Rc::ptr_eq(g, gv),
        None => Rc::ptr_eq(&vp.gvenv, gv),
    };
    let ok = same_env && same_module && !c.variadic && c.skip == 0 && c.params.len() == n
        && c.body.as_ref().and_then(|b| b.func_id) == Some(fid);
    if ok {
        m.nctx.budget -= 1;
    }
    ok
}

/// Return the values to the LCont or CChain, as the VM does for a Body
/// without an LCont (`__NULL__ a ...`, or a Boolean clause that is missing).
/// Values for the Rust call that is running go back to it at once (as
/// `apply` would send them next).
pub fn fused_return<const N: usize>(m: &Machine, vp: &mut VProc, env: LVEnv, sinfo: &Option<SInfo>, values: [Value; N]) -> NResult {
    vp.olist.clear();
    vp.olist.extend(values);
    vp.operator = Value::Null;
    vp.lcont = None;
    vp.sinfo = sinfo.clone();
    vp.lvenv = env;
    Ok(to_caller(m, vp))
}

/// After a chain has put values in the OList with null as the Operator and
/// no LCont: Next::Return if they are for the Rust call that is running
/// (the CChain is the one it was called with), else Next::Apply.
#[inline]
fn to_caller(m: &Machine, vp: &VProc) -> Next {
    match m.nctx.frames.last() {
        Some(f) if same_chain(&vp.cchain, &f.base) => Next::Return,
        _ => Next::Apply,
    }
}

/// `finish` without an LCont, at the end of a chain: the values go back to
/// the Rust call at once if they are for it.
pub fn finish_return(m: &Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, out: Out) -> NResult {
    let r = finish(vp, env, gv, t, None, out)?;
    Ok(match r {
        Next::Apply if vp.operator.is_null() && vp.lcont.is_none() => to_caller(m, vp),
        r => r,
    })
}

/// A call without an LCont: leave it to `apply` (which counts it against the
/// budget) as the VM would do it.
pub fn fused_tail_call<const N: usize>(vp: &mut VProc, env: LVEnv, sinfo: &Option<SInfo>, f: Value, args: [Value; N]) -> NResult {
    vp.olist.clear();
    vp.olist.extend(args);
    vp.operator = f;
    vp.lcont = None;
    vp.sinfo = sinfo.clone();
    vp.lvenv = env;
    Ok(Next::Apply)
}

/// The f64 of an operand of a Number primitive in Function fid's Body, with
/// the error of the primitive (at that Body) if it is not a Number.
#[inline(always)]
pub fn chk(vp: &mut VProc, t: &Table, fid: usize, name: &str, v: &Value) -> Result<f64, VmError> {
    match v {
        Value::Number(x) => Ok(*x),
        _ => chk_error(vp, t, fid, name, v),
    }
}

#[cold]
#[inline(never)]
fn chk_error(vp: &mut VProc, t: &Table, fid: usize, name: &str, v: &Value) -> Result<f64, VmError> {
    vp.sinfo = body(t, fid).sinfo.clone();
    number::check_number(name, v)
}

/// __Dict_lookup_or__ with a String key written in place (operand 1 of the
/// Body of Function fid), whose text (without escapes) the transpiler gives:
/// the value, or None if the key is missing (the chain then applies
/// onMissing, which it evaluates only then).
#[inline]
pub fn d_dict_lookup_str(t: &Table, fid: usize, d: &Value, text: &str, hash: u64) -> Result<Option<Value>, VmError> {
    let Value::Dict(o) = d else { return collections::dict_lookup_core(d, cref(t, fid, 1)) };
    Ok(o.find_str_hashed(text, hash).map(|i| o.entries.borrow()[i].1.clone()))
}

/// __Dict_lookup_or__ with a String key written in place, for a value that the
/// chain uses only as conditions: whether it is true, tested in the Dict
/// without cloning it. None if d is not a Dict, the key is missing or the
/// value is not a Boolean (the code of the Body does the lookup then).
#[inline]
pub fn d_dict_lookup_bool_str(d: &Value, text: &str, hash: u64) -> Option<bool> {
    let Value::Dict(o) = d else { return None };
    let i = o.find_str_hashed(text, hash)?;
    boolean_choice(&o.entries.borrow()[i].1)
}

/// __Dict_put__ with a String key written in place (operand 1 of the Body of
/// Function fid).
#[inline]
pub fn d_dict_put_str(t: &Table, fid: usize, d: &Value, text: &str, hash: u64, value: &Value) -> Result<(), VmError> {
    let Value::Dict(o) = d else { return collections::dict_put_core(d, cref(t, fid, 1).clone(), value.clone()) };
    let found = o.find_str_hashed(text, hash);
    match found {
        Some(i) => o.entries.borrow_mut()[i].1 = value.clone(),
        None => o.append(cref(t, fid, 1).clone(), value.clone()).map_err(VmError::Error)?,
    }
    Ok(())
}
