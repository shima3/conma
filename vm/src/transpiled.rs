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

#![allow(dead_code)] // without generated code, the helpers are unused

use crate::module::{make_closure, FuncDef, GVEnv};
use crate::prim::control::boolean_choice;
use crate::prim::{PResult, PrimDef, VmError};
use crate::sched::Machine;
use crate::step::{eval, load_body_interp};
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// What `run` does next.
pub enum Next {
    /// Run the Body of this Function in this LVEnv.
    Enter(usize, LVEnv),
    /// The VProc is as before a step of the VM: do that step if it is one
    /// that `apply` does.
    Apply,
}

pub type NResult = Result<Next, VmError>;

pub type TranspiledFn = fn(&mut Machine, &mut VProc, LVEnv, &Rc<GVEnv>, &Table) -> NResult;

/// The steps `run` does in one step of the VM at most, so that a VProc that
/// loops in transpiled code lets the others run.
const BUDGET: u32 = 1000;

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
}

/// The transpiled code of a loaded module, with the Functions it refers to.
pub struct Table {
    pub funcs: Vec<Rc<FuncDef>>,
    pub fns: &'static [Option<TranspiledFn>],
    pub prims: Vec<&'static PrimDef>,
    /// The PrimDefs of SAFE_PRIMS.
    safe: Vec<*const PrimDef>,
}

mod generated {
    #![allow(unused_variables)]
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
    if m.nfuncs != funcs.len() || m.fns.len() != funcs.len() {
        return None;
    }
    let mut prims = Vec::new();
    for name in m.prims {
        prims.push(crate::prim::lookup(name)?);
    }
    let mut safe: Vec<*const PrimDef> = SAFE_PRIMS.iter().filter_map(|n| crate::prim::lookup(n)).map(|p| p as *const PrimDef).collect();
    safe.sort();
    Some(Rc::new(Table { funcs, fns: m.fns, prims, safe }))
}

// ── The loop ──

/// Run the Body of Function fid in env (load_body calls this instead of
/// loading it), and the steps after it that `apply` does, up to BUDGET.
pub fn run(m: &mut Machine, vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {
    let mut next = Next::Enter(fid, env);
    let mut budget = BUDGET;
    loop {
        next = match next {
            Next::Enter(fid, env) => match t.fns[fid] {
                Some(f) => f(m, vp, env, gv, t)?,
                None => deopt(vp, fid, env, gv, t)?,
            },
            Next::Apply => {
                if budget == 0 {
                    return Ok(());
                }
                budget -= 1;
                match apply(m, vp, gv, t)? {
                    Some(n) => n,
                    None => return Ok(()),
                }
            }
        };
    }
}

/// Do the next step of the VM if it is one of these, as `step` would do it:
/// null as the Operator (pass the OList to the LCont, or to the top CFrame of
/// the CChain), a Closure of this module with as many operands as its
/// parameters (bind them; the LCont goes to the CChain), an escape
/// continuation (set the CChain back), a primitive of SAFE_PRIMS. None for
/// anything else: the VM does that step.
fn apply(m: &mut Machine, vp: &mut VProc, gv: &Rc<GVEnv>, t: &Table) -> Result<Option<Next>, VmError> {
    match &vp.operator {
        Value::Null => {
            if let Some(lc) = vp.lcont.take() {
                vp.operator = lc;
                return Ok(Some(Next::Apply));
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
            let same_module = match &c.gvenv {
                Some(g) => Rc::ptr_eq(g, gv),
                None => Rc::ptr_eq(&vp.gvenv, gv),
            };
            let Some(fid) = c.body.as_ref().and_then(|b| b.func_id) else { return Ok(None) };
            if !same_module {
                return Ok(None);
            }
            let c = c.clone();
            let params = c.params();
            let mut env = c.lvenv.clone();
            for (k, v) in vp.olist.drain(..).enumerate() {
                env = lv_push(&env, params[k].clone(), v);
            }
            if let Some(lc) = vp.lcont.take() {
                let sinfo = vp.sinfo.clone();
                vp.cchain_push(lc, sinfo);
            }
            vp.operator = Value::Null;
            Ok(Some(Next::Enter(fid, env)))
        }
        Value::Cont(k) => {
            let Some((cf, rest)) = crate::vproc::cchain_split(&k.cchain) else { return Ok(None) };
            vp.lcont = None;
            vp.cchain = rest;
            vp.operator = cf.closure.clone();
            vp.sinfo = cf.sinfo.clone();
            Ok(Some(Next::Apply))
        }
        Value::Prim(p) => {
            let p: &'static PrimDef = *p;
            if t.safe.binary_search(&(p as *const PrimDef)).is_err() {
                return Ok(None);
            }
            (p.f)(m, vp)?;
            Ok(Some(Next::Apply))
        }
        _ => Ok(None),
    }
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

/// `__NULL__ a ... ,(x ...) ...`: pass the operands to the LCont Function lc.
pub fn null_pass(vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, lc: usize) -> NResult {
    load_operands(vp, fid, &env, gv, t)?;
    vp.sinfo = body(t, fid).sinfo.clone();
    vp.operator = Value::Null;
    vp.lcont = None;
    pass_results(vp, env, gv, t, Some(lc))
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
    // The thunk is applied with the LCont: the LCont goes to the CChain.
    if let Some(l) = lc {
        let lcv = make_closure(&t.funcs[l], &env, Some(gv));
        vp.cchain_push(lcv, b.sinfo.clone());
    }
    vp.lcont = None;
    vp.operator = Value::Null;
    Ok(Next::Enter(cf, env))
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

pub fn d_is_null(m: &mut Machine, a: Value) -> Result<Out, VmError> {
    Ok(Out::R1(basic::is_null_core(m, &a)))
}
pub fn d_type_name(_m: &mut Machine, a: Value) -> Result<Out, VmError> {
    Ok(Out::R1(basic::type_name_core(&a)?))
}
pub fn d_number_add(_m: &mut Machine, a: Value, b: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_add__", &a, &b, |x, y| x + y)?))
}
pub fn d_number_subtract(_m: &mut Machine, a: Value, b: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_subtract__", &a, &b, |x, y| x - y)?))
}
pub fn d_number_multiply(_m: &mut Machine, a: Value, b: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::binop_core("__Number_multiply__", &a, &b, |x, y| x * y)?))
}
pub fn d_number_is_zero(m: &mut Machine, a: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_zero__", &a, |x| x == 0.0)?))
}
pub fn d_number_is_positive(m: &mut Machine, a: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_positive__", &a, |x| x > 0.0)?))
}
pub fn d_number_is_negative(m: &mut Machine, a: Value) -> Result<Out, VmError> {
    Ok(Out::R1(number::predicate_core(m, "__Number_is_negative__", &a, |x| x < 0.0)?))
}
pub fn d_string_equal(m: &mut Machine, a: Value, b: Value) -> Result<Out, VmError> {
    Ok(Out::R1(string::equal_core(m, &a, &b)?))
}
pub fn d_string_concat(_m: &mut Machine, a: Value, b: Value) -> Result<Out, VmError> {
    Ok(Out::R1(string::concat_core(&a, &b)?))
}
pub fn d_llist_cons(_m: &mut Machine, head: Value, tail: Value) -> Result<Out, VmError> {
    Ok(Out::R1(Value::LList(Rc::new(LList { head, tail }))))
}
pub fn d_llist_uncons(_m: &mut Machine, on_error: Value, lst: Value) -> Result<Out, VmError> {
    Ok(match llist::uncons_core(&lst) {
        Ok((h, t)) => Out::R2(h, t),
        Err(msg) => Out::Call(on_error, string_value(&msg)),
    })
}
pub fn d_dict_lookup_or(_m: &mut Machine, d: Value, key: Value, on_missing: Value) -> Result<Out, VmError> {
    Ok(match collections::dict_lookup_core(&d, &key)? {
        Some(v) => Out::R1(v),
        None => Out::Call(on_missing, key),
    })
}
pub fn d_dict_put(_m: &mut Machine, d: Value, key: Value, value: Value) -> Result<Out, VmError> {
    collections::dict_put_core(&d, key, value)?;
    Ok(Out::R0)
}

/// `__Cont_get__ ,(return) ...`: the escape continuation of the calling
/// function, passed to the LCont written in place.
pub fn cont_get(m: &mut Machine, vp: &mut VProc) -> Result<Out, VmError> {
    m.serials.conts += 1;
    Ok(Out::R1(Value::Cont(Rc::new(ContObj { serial: m.serials.conts, cchain: vp.cchain.clone() }))))
}
