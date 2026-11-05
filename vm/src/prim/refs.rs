//! Ref and PDict primitives.

use super::{err, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;

pub fn ref_new(m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Ref_new__", 1)?;
    m.serials.refs += 1;
    let r = Value::Ref(crate::gc::new_ref(RefObj::new(m.serials.refs, a.pop().unwrap())));
    vp.prim_return(vec![r]);
    Ok(())
}

pub fn ref_get(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__Ref_get__", 1)?;
    match &a[0] {
        Value::Ref(r) => {
            let v = r.value.borrow().clone();
            vp.prim_return(vec![v]);
            Ok(())
        }
        v => err(format!("__Ref_get__: expected Ref, got {}", value_type_name(v))),
    }
}

pub fn ref_set(m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Ref_set__", 2)?;
    let value = a.pop().unwrap();
    match &a[0] {
        Value::Ref(r) => {
            *r.value.borrow_mut() = value;
            m.ref_changed = true;
            vp.prim_return(vec![]);
            Ok(())
        }
        v => err(format!("__Ref_set__: expected Ref, got {}", value_type_name(v))),
    }
}

pub fn pdict_put(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__PDict_put__", 2)?;
    let value = a.pop().unwrap();
    let key = a.pop().unwrap();
    let mut found = None;
    for (i, (k, _)) in vp.pdict.iter().enumerate() {
        if values_equal(k, &key).map_err(VmError::Error)? {
            found = Some(i);
            break;
        }
    }
    match found {
        Some(i) => vp.pdict[i].1 = value,
        None => vp.pdict.push((key, value)),
    }
    vp.prim_return(vec![]);
    Ok(())
}

/// If the key is missing, onMissing is applied to the key with the same LCont.
pub fn pdict_lookup_or(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__PDict_lookup_or__", 2)?;
    let on_missing = a.pop().unwrap();
    let key = a.pop().unwrap();
    let mut found = None;
    for (k, v) in vp.pdict.iter() {
        if values_equal(k, &key).map_err(VmError::Error)? {
            found = Some(v.clone());
            break;
        }
    }
    match found {
        Some(v) => vp.prim_return(vec![v]),
        None => {
            vp.operator = on_missing;
            vp.olist = vec![key];
        }
    }
    Ok(())
}
