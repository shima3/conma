//! __TRUE__, __FALSE__, __if_then_else__, __if__, __unless__,
//! __if_null_then_else__, __unless_null__, __Number_if_zero_then_else__ and
//! __Number_unless_zero__: the ConMa definitions of include/ref/control.se (and of
//! a test with __is_null__ or __Number_is_zero__ before them) in fewer steps.
//!
//! A Boolean is one of the two Closures `,(t f) __NULL__ t` and
//! `,(t f) __NULL__ f` (as the primitives return them in concise ConMa, and as
//! include/ref/pure.se defines __TRUE__ and __FALSE__), or one of the
//! primitives __TRUE__ and __FALSE__ of fast ConMa (the feature "boolean",
//! which the primitives then return); it chooses its clause at once. Any other value is
//! applied to the two clauses, as the ConMa definition does, and the clause it
//! passes back is applied.

use super::{check_n, take_at, PResult};
use crate::module::CExpr;
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

/// Some(true) for __TRUE__ and `,(t f) __NULL__ t`, Some(false) for __FALSE__
/// and `,(t f) __NULL__ f`, None for any other value.
pub fn boolean_choice(v: &Value) -> Option<bool> {
    let c = match v {
        Value::Prim(p) => {
            let f = p.f as *const ();
            return if f == true_ as fn(&mut Machine, &mut VProc) -> PResult as *const () {
                Some(true)
            } else if f == false_ as fn(&mut Machine, &mut VProc) -> PResult as *const () {
                Some(false)
            } else {
                None
            };
        }
        Value::Closure(c) => c,
        _ => return None,
    };
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

/// Apply the clause chosen by b (then if b) with the LCont: __nop__ returns
/// nothing to the LCont.
fn choose_clause(m: &mut Machine, vp: &mut VProc, b: bool, then: Value, els: Value) {
    vp.olist.clear();
    let clause = if b { then } else { els };
    if m.is_nop(&clause) {
        vp.prim_return0();
    } else {
        vp.operator = clause; // applied with the LCont
    }
}

fn choose(m: &mut Machine, vp: &mut VProc, boolean: Value, then: Value, els: Value) -> PResult {
    vp.olist.clear();
    match boolean_choice(&boolean) {
        Some(b) => choose_clause(m, vp, b, then, els),
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

/// __if_null_then_else__ Value then else: then if Value is __NULL__, else
/// otherwise (`__is_null__ Value ,(b) __if_then_else__ b then else` in one step).
pub fn if_null_then_else(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__if_null_then_else__", 3)?;
    let els = take_at(vp, 2);
    let then = take_at(vp, 1);
    let v = take_at(vp, 0);
    choose_clause(m, vp, v.is_null(), then, els);
    Ok(())
}

/// __unless_null__ Value else: else unless Value is __NULL__
/// (`__is_null__ Value ,(b) __unless__ b else` in one step).
pub fn unless_null(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__unless_null__", 2)?;
    let els = take_at(vp, 1);
    let v = take_at(vp, 0);
    let nop = m.nop_closure();
    choose_clause(m, vp, v.is_null(), nop, els);
    Ok(())
}

/// __Number_if_zero_then_else__ Number then else: then if Number is 0, else otherwise
/// (`__Number_is_zero__ Number ,(b) __if_then_else__ b then else` in one step);
/// an error, as __Number_is_zero__ gives, if it is not a Number.
pub fn number_if_zero_then_else(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__Number_if_zero_then_else__", 3)?;
    let x = match crate::prim::number::check_number("__Number_if_zero_then_else__", &vp.olist[0]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 3, e),
    };
    let els = take_at(vp, 2);
    let then = take_at(vp, 1);
    choose_clause(m, vp, x == 0.0, then, els);
    Ok(())
}

/// __Number_unless_zero__ Number else: else unless Number is 0
/// (`__Number_is_zero__ Number ,(b) __unless__ b else` in one step).
pub fn number_unless_zero(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__Number_unless_zero__", 2)?;
    let x = match crate::prim::number::check_number("__Number_unless_zero__", &vp.olist[0]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 2, e),
    };
    let els = take_at(vp, 1);
    let nop = m.nop_closure();
    choose_clause(m, vp, x == 0.0, nop, els);
    Ok(())
}

/// __Number_if_greater__ Number Number then: then if the first Number is greater
/// than the second, nothing otherwise (`__Number_subtract__ a b ,(d)
/// __Number_is_positive__ d ,(g) __if__ g then` in one step); an error if one is not
/// a Number.
pub fn number_if_greater(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__Number_if_greater__", 3)?;
    let a = match crate::prim::number::check_number("__Number_if_greater__", &vp.olist[0]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 3, e),
    };
    let b = match crate::prim::number::check_number("__Number_if_greater__", &vp.olist[1]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 3, e),
    };
    let then = take_at(vp, 2);
    let nop = m.nop_closure();
    choose_clause(m, vp, a > b, then, nop);
    Ok(())
}

/// __Number_if_greater_then_else__ Number Number then else: then if the first
/// Number is greater than the second, else otherwise (`__Number_subtract__ a b ,(d)
/// __Number_is_positive__ d ,(g) __if_then_else__ g then else` in one step); an
/// error if one is not a Number.
pub fn number_if_greater_then_else(m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__Number_if_greater_then_else__", 4)?;
    let a = match crate::prim::number::check_number("__Number_if_greater_then_else__", &vp.olist[0]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 4, e),
    };
    let b = match crate::prim::number::check_number("__Number_if_greater_then_else__", &vp.olist[1]) {
        Ok(v) => v,
        Err(e) => return crate::prim::in_place_failed(vp, 4, e),
    };
    let els = take_at(vp, 3);
    let then = take_at(vp, 2);
    choose_clause(m, vp, a > b, then, els);
    Ok(())
}

/// __TRUE__ then else ,(then): passes then to the LCont, without applying it.
pub fn true_(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__TRUE__", 2)?;
    let then = take_at(vp, 0);
    vp.prim_return1(then);
    Ok(())
}

/// __FALSE__ then else ,(else): passes else to the LCont, without applying it.
pub fn false_(_m: &mut Machine, vp: &mut VProc) -> PResult {
    check_n(vp, "__FALSE__", 2)?;
    let els = take_at(vp, 1);
    vp.prim_return1(els);
    Ok(())
}
