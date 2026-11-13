//! MSeq and Dict primitives.

use super::{err, take_n as take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

fn new_mseq(m: &mut Machine, items: Vec<Value>) -> Value {
    m.serials.mseqs += 1;
    Value::MSeq(crate::gc::new_mseq(MSeqObj::new(m.serials.mseqs, items)))
}

/// Take onError, an MSeq and count-2 more operands. None after applying
/// onError if the second is not an MSeq.
fn mseq_args(vp: &mut VProc, name: &str, count: usize) -> Result<Option<(Value, Rc<MSeqObj>, Vec<Value>)>, VmError> {
    let mut a = take_args(vp, name, count)?;
    let rest = a.split_off(2);
    let mseq = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    match mseq {
        Value::MSeq(s) => Ok(Some((on_error, s, rest))),
        other => {
            vp.prim_error(on_error, &format!("{}: expected MSeq, got {}", name, value_type_name(&other)));
            Ok(None)
        }
    }
}

pub fn mseq_new(m: &mut Machine, vp: &mut VProc) -> PResult {
    let s = new_mseq(m, Vec::new());
    vp.prim_return(vec![s]);
    Ok(())
}

pub fn mseq_is_empty(m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((_, s, _)) = mseq_args(vp, "__MSeq_is_empty__", 2)? {
        let b = if s.is_empty() { m.true_closure() } else { m.false_closure() };
        vp.prim_return(vec![b]);
    }
    Ok(())
}

pub fn mseq_uncons(m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((on_error, s, _)) = mseq_args(vp, "__MSeq_uncons__", 2)? {
        let Some(head) = s.head() else {
            vp.prim_error(on_error, "__MSeq_uncons__: empty MSeq");
            return Ok(());
        };
        // The rest shares the items (an MSeq copies them before changing them).
        m.serials.mseqs += 1;
        let tail = Value::MSeq(crate::gc::new_mseq(s.rest(m.serials.mseqs)));
        vp.prim_return(vec![head, tail]);
    }
    Ok(())
}

pub fn mseq_set_head(_m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((on_error, s, mut rest)) = mseq_args(vp, "__MSeq_set_head__", 3)? {
        let value = rest.pop().unwrap();
        if s.is_empty() {
            vp.prim_error(on_error, "__MSeq_set_head__: empty MSeq");
            return Ok(());
        }
        s.items_mut()[0] = value;
        vp.prim_return(vec![]);
    }
    Ok(())
}

pub fn mseq_append(_m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((_, s, mut rest)) = mseq_args(vp, "__MSeq_append__", 3)? {
        s.items_mut().push(rest.pop().unwrap());
        vp.prim_return(vec![]);
    }
    Ok(())
}

/// Take a Dict and count-1 more operands; fail if the first is not a Dict.
fn dict_args(vp: &mut VProc, name: &str, count: usize) -> Result<(Rc<DictObj>, Vec<Value>), VmError> {
    let mut a = take_args(vp, name, count)?;
    let rest = a.split_off(1);
    match a.pop().unwrap() {
        Value::Dict(d) => Ok((d, rest)),
        other => err(format!("{}: expected Dict, got {}", name, value_type_name(&other))),
    }
}

pub fn dict_new(m: &mut Machine, vp: &mut VProc) -> PResult {
    m.serials.dicts += 1;
    let d = Value::Dict(crate::gc::new_dict(DictObj::new(m.serials.dicts)));
    vp.prim_return(vec![d]);
    Ok(())
}

/// The position of key in d, through the DictSite of the Body that applies the primitive when key
/// is the String constant of that place (VProc::dict_site, module::DictSite): the cache, or the
/// index without making the text of the key again. Otherwise DictObj::find.
#[inline]
fn find_at_site(site: Option<Rc<crate::module::DictSite>>, d: &DictObj, key: &Value) -> Result<Option<usize>, VmError> {
    if let (Some(s), Value::StrNode(crate::sexp::Sexp::List(k))) = (&site, key) {
        if let crate::sexp::Sexp::List(n) = &s.node {
            if Rc::ptr_eq(k, n) {
                return Ok(d.find_str_cached(&s.text, s.hash, &s.ic));
            }
        }
    }
    d.find(key).map_err(VmError::Error)
}

pub fn dict_put(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let site = vp.dict_site.take();
    // The usual case, without taking the operands into new Vecs: the OList is used for the
    // results (as prim_return would set it)
    if vp.olist.len() >= 3 {
        if let Value::Dict(d) = &vp.olist[0] {
            let d = d.clone();
            let found = match find_at_site(site, &d, &vp.olist[1]) {
                Ok(f) => f,
                Err(e) => {
                    vp.olist.drain(..3);
                    return Err(e);
                }
            };
            let value = std::mem::replace(&mut vp.olist[2], Value::Null);
            match found {
                None => {
                    let key = std::mem::replace(&mut vp.olist[1], Value::Null);
                    if let Err(e) = d.append(key, value) {
                        vp.olist.drain(..3);
                        return Err(VmError::Error(e));
                    }
                }
                Some(i) => d.entries_mut()[i].1 = value,
            }
            vp.olist.clear();
            vp.operator = vp.lcont.take().unwrap_or(Value::Null);
            return Ok(());
        }
    }
    let (d, mut a) = dict_args(vp, "__Dict_put__", 3)?;
    let value = a.pop().unwrap();
    let key = a.pop().unwrap();
    match find_at_site(site, &d, &key)? {
        None => d.append(key, value).map_err(VmError::Error)?,
        Some(i) => d.entries_mut()[i].1 = value,
    }
    vp.prim_return(vec![]);
    Ok(())
}

/// The value of key in the Dict d, or None (for vm/src/transpiled.rs; the
/// checks and errors of __Dict_lookup_or__).
pub fn dict_lookup_core(d: &Value, key: &Value) -> Result<Option<Value>, VmError> {
    let Value::Dict(d) = d else {
        return err(format!("__Dict_lookup_or__: expected Dict, got {}", value_type_name(d)));
    };
    Ok(d.find(key).map_err(VmError::Error)?.map(|i| d.entries.borrow()[i].1.clone()))
}

/// Put value under key in the Dict d (for vm/src/transpiled.rs; the checks and
/// errors of __Dict_put__).
pub fn dict_put_core(d: &Value, key: Value, value: Value) -> Result<(), VmError> {
    let Value::Dict(d) = d else {
        return err(format!("__Dict_put__: expected Dict, got {}", value_type_name(d)));
    };
    match d.find(&key).map_err(VmError::Error)? {
        None => d.append(key, value).map_err(VmError::Error)?,
        Some(i) => d.entries_mut()[i].1 = value,
    }
    Ok(())
}

pub fn dict_lookup_or(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let site = vp.dict_site.take();
    // The usual case, without taking the operands into new Vecs: the OList is used for the
    // result or for the key given to onMissing (as prim_return and the code below would set it)
    if vp.olist.len() >= 3 {
        if let Value::Dict(d) = &vp.olist[0] {
            let found = match find_at_site(site, d, &vp.olist[1]) {
                Ok(f) => f,
                Err(e) => {
                    vp.olist.drain(..3);
                    return Err(e);
                }
            };
            match found {
                Some(i) => {
                    let v = d.entries.borrow()[i].1.clone();
                    vp.olist.clear();
                    vp.olist.push(v);
                    vp.operator = vp.lcont.take().unwrap_or(Value::Null);
                }
                None => {
                    let key = std::mem::replace(&mut vp.olist[1], Value::Null);
                    vp.operator = std::mem::replace(&mut vp.olist[2], Value::Null);
                    vp.olist.clear();
                    vp.olist.push(key);
                }
            }
            return Ok(());
        }
    }
    let (d, mut a) = dict_args(vp, "__Dict_lookup_or__", 3)?;
    let on_missing = a.pop().unwrap();
    let key = a.pop().unwrap();
    match find_at_site(site, &d, &key)? {
        Some(i) => {
            let v = d.entries.borrow()[i].1.clone();
            vp.prim_return(vec![v]);
        }
        None => {
            vp.operator = on_missing;
            vp.olist = vec![key];
        }
    }
    Ok(())
}

pub fn dict_remove(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, a) = dict_args(vp, "__Dict_remove__", 2)?;
    if let Some(i) = d.find(&a[0]).map_err(VmError::Error)? {
        d.remove(i).map_err(VmError::Error)?;
    }
    vp.prim_return(vec![]);
    Ok(())
}

/// __Dict_copy__ Dict ,(Dict): a new Dict with the same keys and values, in the same order.
pub fn dict_copy(m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, _) = dict_args(vp, "__Dict_copy__", 1)?;
    m.serials.dicts += 1;
    // The entries and the index are shared until one of the two is changed.
    let c = Value::Dict(crate::gc::new_dict(d.copy(m.serials.dicts)));
    vp.prim_return(vec![c]);
    Ok(())
}

pub fn dict_keys(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, _) = dict_args(vp, "__Dict_keys__", 1)?;
    let keys: Vec<Value> = d.entries.borrow().iter().map(|(k, _)| k.clone()).collect();
    vp.prim_return(vec![Value::Seq(Rc::new(SeqClosure { olist: keys, sinfo: None, lvenv: None, scope: None }))]);
    Ok(())
}
