//! MSeq and Dict primitives.

use super::{err, take_n as take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn new_mseq(m: &mut Machine, items: Vec<Value>) -> Value {
    m.serials.mseqs += 1;
    Value::MSeq(crate::gc::new_mseq(MSeqObj { serial: m.serials.mseqs, items: RefCell::new(items) }))
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
        let b = if s.items.borrow().is_empty() { m.true_closure() } else { m.false_closure() };
        vp.prim_return(vec![b]);
    }
    Ok(())
}

pub fn mseq_uncons(m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((on_error, s, _)) = mseq_args(vp, "__MSeq_uncons__", 2)? {
        let items = s.items.borrow();
        if items.is_empty() {
            drop(items);
            vp.prim_error(on_error, "__MSeq_uncons__: empty MSeq");
            return Ok(());
        }
        let head = items[0].clone();
        let rest = items[1..].to_vec();
        drop(items);
        let tail = new_mseq(m, rest);
        vp.prim_return(vec![head, tail]);
    }
    Ok(())
}

pub fn mseq_set_head(_m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((on_error, s, mut rest)) = mseq_args(vp, "__MSeq_set_head__", 3)? {
        let value = rest.pop().unwrap();
        if s.items.borrow().is_empty() {
            vp.prim_error(on_error, "__MSeq_set_head__: empty MSeq");
            return Ok(());
        }
        s.items.borrow_mut()[0] = value;
        vp.prim_return(vec![]);
    }
    Ok(())
}

pub fn mseq_append(_m: &mut Machine, vp: &mut VProc) -> PResult {
    if let Some((_, s, mut rest)) = mseq_args(vp, "__MSeq_append__", 3)? {
        s.items.borrow_mut().push(rest.pop().unwrap());
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
    let d = Value::Dict(crate::gc::new_dict(DictObj {
        serial: m.serials.dicts,
        entries: RefCell::new(Vec::new()),
        index: RefCell::new(HashMap::new()),
    }));
    vp.prim_return(vec![d]);
    Ok(())
}

pub fn dict_put(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, mut a) = dict_args(vp, "__Dict_put__", 3)?;
    let value = a.pop().unwrap();
    let key = a.pop().unwrap();
    match d.find(&key).map_err(VmError::Error)? {
        None => d.append(key, value).map_err(VmError::Error)?,
        Some(i) => d.entries.borrow_mut()[i].1 = value,
    }
    vp.prim_return(vec![]);
    Ok(())
}

pub fn dict_lookup_or(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, mut a) = dict_args(vp, "__Dict_lookup_or__", 3)?;
    let on_missing = a.pop().unwrap();
    let key = a.pop().unwrap();
    match d.find(&key).map_err(VmError::Error)? {
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

pub fn dict_keys(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let (d, _) = dict_args(vp, "__Dict_keys__", 1)?;
    let keys: Vec<Value> = d.entries.borrow().iter().map(|(k, _)| k.clone()).collect();
    vp.prim_return(vec![Value::Seq(Rc::new(SeqClosure { olist: keys, sinfo: None, lvenv: None }))]);
    Ok(())
}
