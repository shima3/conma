//! LazyList primitives and __Ref_wait_change__.
//!
//! A LazyList is a Ref whose value is a thunk (a Closure), null, an LList
//! cell, or the VProc that is evaluating it. While a VProc evaluates it, the
//! Ref keeps the CChain of the VProc when it claimed the LazyList (None for a
//! LazyList made by __LazyList_hold__) and the thunk (LazyInfo).

use super::{err, take_n, PResult};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::{State, VProc, VProcCell};
use std::rc::Rc;

/// True if the value is this VProc.
fn is_vproc(v: &Value, cell: &VProcCell) -> bool {
    matches!(v, Value::VProc(o) if std::ptr::eq(Rc::as_ptr(o), cell))
}

/// `a is b` for two CChains.
fn same_chain(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::LList(x), Value::LList(y)) => Rc::ptr_eq(x, y),
        (Value::LList(_), _) | (_, Value::LList(_)) => false,
        _ => a.is_null() && b.is_null(),
    }
}

/// True if the VProc that claimed the LazyList has left its evaluation: the
/// CChain at the claim is no longer the bottom of owner_cchain, the CChain of
/// that VProc. A LazyList made by __LazyList_hold__ is never abandoned.
pub fn lazy_abandoned(r: &RefObj, owner_cchain: &Value) -> bool {
    let chain = match &*r.lazy.borrow() {
        Some(LazyInfo { chain: Some(c), .. }) => c.clone(),
        _ => return false,
    };
    let mut node = owner_cchain.clone();
    while !same_chain(&node, &chain) {
        match node {
            Value::LList(cell) => node = cell.tail.clone(),
            _ => return true,
        }
    }
    false
}

/// True if the VProc is evaluating a LazyList that it claimed and has not
/// left (a LazyList made by __LazyList_hold__ does not count). Forgets the
/// LazyLists that it no longer evaluates.
pub fn evaluating_lazy(cell: &VProcCell, vp: &mut VProc) -> bool {
    let cchain = vp.cchain.clone();
    vp.claims.retain(|r| {
        is_vproc(&r.value.borrow(), cell)
            && matches!(&*r.lazy.borrow(), Some(LazyInfo { chain: Some(_), .. }))
            && !lazy_abandoned(r, &cchain)
    });
    !vp.claims.is_empty()
}

/// __LazyList_claim__ onError LazyList ,(thunk cell)
pub fn claim(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__LazyList_claim__";
    if vp.olist.len() < 2 {
        return err(format!("{}: missing argument(s)", name));
    }
    let on_error = vp.olist[0].clone();
    let lazy = match &vp.olist[1] {
        Value::Ref(r) => r.clone(),
        other => {
            let msg = format!("{}: expected LazyList, got {}", name, value_type_name(other));
            vp.olist.drain(..2);
            vp.prim_error(on_error, &msg);
            return Ok(());
        }
    };
    let cur = m.current.clone().expect("no current VProc");
    let mut value = lazy.value.borrow().clone();
    if let Value::VProc(owner) = &value {
        if owner.state.get() == State::Finished {
            vp.olist.drain(..2);
            let msg = format!("{}: VProc {} finished without evaluating the LazyList", name, owner.serial);
            vp.prim_error(on_error, &msg);
            return Ok(());
        }
        let own = Rc::ptr_eq(owner, &cur);
        let abandoned = if own {
            lazy_abandoned(&lazy, &vp.cchain)
        } else {
            lazy_abandoned(&lazy, &owner.vp.borrow().cchain)
        };
        if !abandoned {
            if own {
                vp.olist.drain(..2);
                let held = matches!(&*lazy.lazy.borrow(), Some(LazyInfo { chain: None, .. }));
                let msg = if held {
                    format!("{}: the LazyList has no value yet and is held by this VProc", name)
                } else {
                    format!("{}: the LazyList depends on itself", name)
                };
                vp.prim_error(on_error, &msg);
            } else {
                // The Operator and OList stay for the retry.
                m.wait_lazy(&cur, &lazy);
            }
            return Ok(());
        }
        value = lazy.lazy.borrow().as_ref().map(|i| i.thunk.clone()).unwrap_or(Value::Null);
    }
    vp.olist.drain(..2);
    if value_type_name(&value) == "Closure" {
        *lazy.value.borrow_mut() = Value::VProc(cur.clone());
        *lazy.lazy.borrow_mut() = Some(LazyInfo { chain: Some(vp.cchain.clone()), thunk: value.clone() });
        // Keep only the LazyLists vp still evaluates, so that claims does not
        // keep evaluated LazyLists alive.
        vp.claims.retain(|r| is_vproc(&r.value.borrow(), &cur));
        vp.claims.push(lazy);
        vp.prim_return(vec![value, Value::Null]);
    } else if value.is_null() || matches!(value, Value::LList(_)) {
        vp.prim_return(vec![Value::Null, value]);
    } else {
        let msg = format!("{}: expected LazyList, got a Ref of {}", name, value_type_name(&value));
        vp.prim_error(on_error, &msg);
    }
    Ok(())
}

/// __LazyList_hold__ ,(LazyList)
pub fn hold(m: &mut Machine, vp: &mut VProc) -> PResult {
    let cur = m.current.clone().expect("no current VProc");
    m.serials.refs += 1;
    let r = RefObj::new(m.serials.refs, Value::VProc(cur));
    *r.lazy.borrow_mut() = Some(LazyInfo { chain: None, thunk: Value::Null });
    vp.prim_return(vec![Value::Ref(Rc::new(r))]);
    Ok(())
}

/// __LazyList_publish__ LazyList cell ,()
pub fn publish(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__LazyList_publish__";
    let mut a = take_n(vp, name, 2)?;
    let cell = a.pop().unwrap();
    let lazy = match a.pop().unwrap() {
        Value::Ref(r) => r,
        other => return err(format!("{}: expected LazyList, got {}", name, value_type_name(&other))),
    };
    let cur = m.current.clone().expect("no current VProc");
    if !is_vproc(&lazy.value.borrow(), &cur) {
        return err(format!("{}: the LazyList is not being evaluated by this VProc", name));
    }
    if !(cell.is_null() || matches!(cell, Value::LList(_))) {
        return err(format!("{}: expected null or LList, got {}", name, value_type_name(&cell)));
    }
    *lazy.value.borrow_mut() = cell;
    m.ref_changed = true;
    *lazy.lazy.borrow_mut() = None;
    vp.claims.retain(|r| !Rc::ptr_eq(r, &lazy));
    vp.prim_return(vec![]);
    if cur.kill_pending.get() && !evaluating_lazy(&cur, vp) {
        m.kill_now(&cur); // a delayed __VProc_kill__
    }
    Ok(())
}

/// __Ref_wait_change__ Ref value ,()
pub fn ref_wait_change(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Ref_wait_change__";
    let mut a = take_n(vp, name, 2)?;
    let value = a.pop().unwrap();
    let r = match a.pop().unwrap() {
        Value::Ref(r) => r,
        other => return err(format!("{}: expected Ref, got {}", name, value_type_name(&other))),
    };
    vp.prim_return(vec![]);
    let unchanged = same(&r.value.borrow(), &value);
    if unchanged {
        let cur = m.current.clone().expect("no current VProc");
        m.wait_change(&cur, &r, value);
    }
    Ok(())
}
