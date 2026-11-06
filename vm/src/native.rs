//! Native code of ConMa modules, made by vm/native/translate.py (stage 2 of
//! item 7 of the ConMa review: running the parts of a program that need no
//! VProc switch as Rust, several steps at a time).
//!
//! A module of native code belongs to one module file, identified by the
//! FNV-1a hash of its text. For each Function of the module (numbered in the
//! order in which load_module compiles them), it may have a native function,
//! which stands for loading the Body of that Function: load_body calls it
//! instead. The native function runs the Body and the Bodies after it as far
//! as it can without the VM (calls of primitives that only work on their
//! operands, passing values on with `__NULL__`, `__if_then_else__`, `__if__`
//! and `__unless__` with clauses written in place), and then either loads a
//! Body the VM goes on with (`deopt`) or leaves the VProc in the state the VM
//! would be in after the same steps. So the results are those of the VM;
//! only the number of steps (and so the order of the VProcs, and traces)
//! differ. With --trace or CONMA_PROFILE the native code is not used.

#![allow(dead_code)] // without generated code, the helpers are unused

use crate::module::{make_closure, FuncDef, GVEnv};
use crate::prim::control::boolean_choice;
use crate::prim::{PResult, PrimDef};
use crate::sched::Machine;
use crate::step::{eval, load_body_interp};
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

pub type NativeFn = fn(&mut Machine, &mut VProc, LVEnv, &Rc<GVEnv>, &Table) -> PResult;

/// The native code of one module file, as the generated code gives it.
pub struct Module {
    pub hash: u64,
    /// The number of Functions in the module.
    pub nfuncs: usize,
    /// The primitives the native code calls, by ConMa name; `prim_call`
    /// takes an index into this list.
    pub prims: &'static [&'static str],
    pub natives: &'static [Option<NativeFn>],
}

/// The native code of a loaded module, with the Functions it refers to.
pub struct Table {
    pub funcs: Vec<Rc<FuncDef>>,
    pub natives: &'static [Option<NativeFn>],
    pub prims: Vec<&'static PrimDef>,
}

mod generated {
    use super::*;
    include!(concat!(env!("OUT_DIR"), "/native_gen.rs"));
}

/// FNV-1a (64 bits) of a module file, as vm/native/translate.py computes it.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// The Table for the module file with this hash, if there is native code for
/// it and it has the same number of Functions.
pub fn table_for(hash: u64, funcs: Vec<Rc<FuncDef>>) -> Option<Rc<Table>> {
    let m = generated::MODULES.iter().find(|m| m.hash == hash)?;
    if m.nfuncs != funcs.len() || m.natives.len() != funcs.len() {
        return None;
    }
    let mut prims = Vec::new();
    for name in m.prims {
        prims.push(crate::prim::lookup(name)?);
    }
    Some(Rc::new(Table { funcs, natives: m.natives, prims }))
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
    t.funcs[fid].body.as_ref().expect("a native Function has a Body")
}

/// Load the Body of Function fid for the VM, without its native code.
pub fn deopt(vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {
    load_body_interp(vp, body(t, fid), env, gv)
}

/// Run the Body of Function fid: its native code if it has some.
pub fn enter(m: &mut Machine, vp: &mut VProc, fid: usize, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {
    match t.natives[fid] {
        Some(f) => f(m, vp, env, gv, t),
        None => deopt(vp, fid, env, gv, t),
    }
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
/// parameters (and not variadic), bind them and run its Body here; otherwise
/// make the Closure and let the VM apply it.
fn pass_results(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: Option<usize>) -> PResult {
    let Some(l) = lc else {
        vp.lvenv = env; // no LCont: the VM goes on with the CChain
        return Ok(());
    };
    let f = &t.funcs[l];
    if f.variadic || vp.olist.len() != f.params.len() {
        vp.operator = make_closure(f, &env, Some(gv));
        vp.lvenv = env;
        return Ok(());
    }
    let mut e = env;
    for (k, v) in vp.olist.drain(..).enumerate() {
        e = lv_push(&e, f.params[k].clone(), v);
    }
    enter(m, vp, l, e, gv, t)
}

/// After a primitive called with the LCont set to a mark (Null): if it
/// returned its results, pass them on; otherwise (onError, a Closure to
/// apply) put the real LCont back and leave the VProc to the VM.
fn after_prim(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, lc: Option<usize>) -> PResult {
    if vp.operator.is_null() && vp.lcont.is_none() {
        return pass_results(m, vp, env, gv, t, lc);
    }
    if matches!(vp.lcont, Some(Value::Null)) {
        vp.lcont = lc.map(|l| make_closure(&t.funcs[l], &env, Some(gv)));
    }
    vp.lvenv = env;
    Ok(())
}

/// `prim a ... [,(x ...) ...]`: the Body of fid calls primitive p (an index
/// into Table::prims) with the LCont Function lc.
pub fn prim_call(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, p: usize, lc: Option<usize>) -> PResult {
    load_operands(vp, fid, &env, gv, t)?;
    let prim = t.prims[p];
    vp.sinfo = body(t, fid).sinfo.clone();
    vp.operator = Value::Prim(prim);
    vp.lcont = Some(Value::Null);
    (prim.f)(m, vp)?;
    after_prim(m, vp, env, gv, t, lc)
}

/// `__NULL__ a ... ,(x ...) ...`: pass the operands to the LCont Function lc.
pub fn null_pass(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, lc: usize) -> PResult {
    load_operands(vp, fid, &env, gv, t)?;
    vp.sinfo = body(t, fid).sinfo.clone();
    vp.operator = Value::Null;
    vp.lcont = None;
    pass_results(m, vp, env, gv, t, Some(lc))
}

/// `__if_then_else__`, `__if__` or `__unless__` in the Body of fid, with the
/// LCont Function lc. Falls back to the VM (deopt) when the Boolean is not
/// one of the two Boolean Closures or the clause is a value other than
/// __nop__.
#[allow(clippy::too_many_arguments)]
pub fn if_call(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table, fid: usize, cond: Cond, then: Clause, els: Clause, lc: Option<usize>) -> PResult {
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
        return pass_results(m, vp, env, gv, t, lc);
    }
    let Clause::Thunk(cf) = clause else { unreachable!() };
    // The thunk is applied with the LCont: the LCont goes to the CChain.
    if let Some(l) = lc {
        let lcv = make_closure(&t.funcs[l], &env, Some(gv));
        vp.cchain_push(lcv, b.sinfo.clone());
    }
    vp.lcont = None;
    vp.operator = Value::Null;
    enter(m, vp, cf, env, gv, t)
}
