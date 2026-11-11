//! __if_then_else__, __if__ and __unless__: the ConMa definitions of
//! include/control.se in fewer steps.
//!
//! A Boolean that is one of the two Closures `,(t f) __NULL__ t` and
//! `,(t f) __NULL__ f` (as the primitives return them, and __TRUE__ and
//! __FALSE__ of pure.se) chooses its clause at once. Any other value is
//! applied to the two clauses, as the ConMa definition does, and the clause it
//! passes back is applied.

use super::{check_n, take_at, PResult};
use crate::module::CExpr;
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

/// Some(true) for `,(t f) __NULL__ t`, Some(false) for `,(t f) __NULL__ f`,
/// None for any other value.
pub fn boolean_choice(v: &Value) -> Option<bool> {
    let Value::Closure(c) = v else { return None };
    if c.variadic || c.params().len() != 2 {
        return None;
    }
    let body = c.body.as_ref()?;
    if !matches!(body.operator, Some(CExpr::C(Value::Null))) || body.lcont.is_some() || body.olist.len() != 1 {
        return None;
    }
    match body.olist[0] {
        CExpr::L(1) => Some(true),
        CExpr::L(0) => Some(false),
        _ => None,
    }
}

fn choose(m: &mut Machine, vp: &mut VProc, boolean: Value, then: Value, els: Value) -> PResult {
    vp.olist.clear();
    match boolean_choice(&boolean) {
        Some(b) => {
            let clause = if b { then } else { els };
            if m.is_nop(&clause) {
                // __nop__ returns nothing to the LCont.
                vp.prim_return0();
            } else {
                vp.operator = clause; // applied with the LCont
            }
        }
        None => {
            // As the ConMa definition: the LCont goes to the CChain, the
            // Boolean is applied to the clauses, and the Closure ,(clause) clause
            // applies the clause it passes back.
            if let Some(lc) = vp.lcont.take() {
                let sinfo = vp.sinfo.clone();
                vp.cchain_push(lc, sinfo);
            }
            vp.operator = boolean;
            vp.olist.push(then);
            vp.olist.push(els);
            vp.lcont = Some(m.select_closure());
        }
    }
    Ok(())
}

/// __if_then_else__ Boolean then else
pub fn if_then_else(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__if_then_else__", 3)?;
    let els = take_at(vp, 2);
    let then = take_at(vp, 1);
    let boolean = take_at(vp, 0);
    choose(m, vp, boolean, then, els)
}

/// __if__ Boolean then
pub fn if_(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__if__", 2)?;
    let then = take_at(vp, 1);
    let boolean = take_at(vp, 0);
    let nop = m.nop_closure();
    choose(m, vp, boolean, then, nop)
}

/// __unless__ Boolean else
pub fn unless(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__unless__", 2)?;
    let els = take_at(vp, 1);
    let boolean = take_at(vp, 0);
    let nop = m.nop_closure();
    choose(m, vp, boolean, nop, els)
}
