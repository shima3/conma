//! The channel primitives of the feature "chan" of extended ConMa
//! (docs/dev/CHAN-PLAN-ja.md, docs/PRIMITIVES.md section 16): several VProcs
//! write to a channel, one chain each, and one reader takes the items. They
//! read and write the same Dicts and Refs as their replacement definitions in
//! include/core/chan.se, which describes them (the keys of the channel, the
//! chains, the writers and the reader), so that each does in one step what
//! the definition does in many, with the same results.
//!
//! A primitive that waits (__Chan_put__ when the channel is full,
//! __Chan_take__ when no item is there) leaves the Operator and the OList as
//! they are and makes the VProc wait for a Ref to change, as
//! __Ref_wait_change__ does; it is applied again when the VProc wakes up, as
//! __LazyList_claim__ is.

use super::control::boolean_choice;
use super::{check_args, err, take_args, PResult, VmError};
use crate::sched::Machine;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// Items taken between two searches for new chains (Chan_core_register_every).
const REGISTER_EVERY: f64 = 16.0;

fn as_dict(name: &str, what: &str, v: &Value) -> Result<Rc<DictObj>, VmError> {
    match v {
        Value::Dict(d) => Ok(d.clone()),
        other => err(format!("{}: expected a {} (a Dict), got {}", name, what, value_type_name(other))),
    }
}

fn get(name: &str, what: &str, d: &DictObj, key: &str) -> Result<Value, VmError> {
    match d.find_str(key) {
        Some(i) => Ok(d.entries.borrow()[i].1.clone()),
        None => err(format!("{}: the {} has no \"{}\"", name, what, key)),
    }
}

fn put(d: &DictObj, key: &str, value: Value) -> Result<(), VmError> {
    match d.find_str(key) {
        Some(i) => d.entries_mut()[i].1 = value,
        None => d.append(string_value(key), value).map_err(VmError::Error)?,
    }
    Ok(())
}

fn as_ref(name: &str, what: &str, v: Value) -> Result<Rc<RefObj>, VmError> {
    match v {
        Value::Ref(r) => Ok(r),
        other => err(format!("{}: expected a Ref as the {}, got {}", name, what, value_type_name(&other))),
    }
}

fn as_number(name: &str, what: &str, v: &Value) -> Result<f64, VmError> {
    match v {
        Value::Number(n) => Ok(*n),
        other => err(format!("{}: expected a Number as the {}, got {}", name, what, value_type_name(other))),
    }
}

fn new_ref(m: &mut Machine, v: Value) -> Value {
    m.serials.refs += 1;
    Value::Ref(crate::gc::new_ref(RefObj::new(m.serials.refs, v)))
}

fn new_dict(m: &mut Machine) -> Rc<DictObj> {
    m.serials.dicts += 1;
    crate::gc::new_dict(DictObj::new(m.serials.dicts))
}

/// Set the value of a Ref, so that the VProcs waiting for it to change wake up.
fn set(m: &mut Machine, r: &RefObj, v: Value) {
    let old = std::mem::replace(&mut *r.value.borrow_mut(), v);
    m.ref_changed = true;
    drop(old);
}

/// A new token for "wake": a new Dict, which no other value is the same as.
fn wake(m: &mut Machine, r: &RefObj) {
    let token = Value::Dict(new_dict(m));
    set(m, r, token);
}

fn list_from(items: Vec<Value>) -> Value {
    items.into_iter().rev().fold(Value::Null, |tail, head| Value::LList(Rc::new(LList { head, tail })))
}

fn list_items(name: &str, mut v: Value) -> Result<Vec<Value>, VmError> {
    let mut items = Vec::new();
    loop {
        match v {
            Value::Null => return Ok(items),
            Value::LList(cell) => {
                items.push(cell.head.clone());
                v = cell.tail.clone();
            }
            other => return err(format!("{}: expected an LList of cursors, got {}", name, value_type_name(&other))),
        }
    }
}

/// __Chan_new__ cap state ,(chan)
pub fn chan_new(m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__Chan_new__", 2)?;
    let state = a.pop().unwrap();
    let cap = a.pop().unwrap();
    let chan = new_dict(m);
    m.serials.mseqs += 1;
    put(&chan, "chains", Value::MSeq(crate::gc::new_mseq(MSeqObj::new(m.serials.mseqs, Vec::new()))))?;
    let token = Value::Dict(new_dict(m));
    let w = new_ref(m, token);
    put(&chan, "wake", w)?;
    let f = m.false_closure();
    let c = new_ref(m, f);
    put(&chan, "cancelled", c)?;
    put(&chan, "cap", cap)?;
    put(&chan, "state", state)?;
    vp.prim_return(vec![Value::Dict(chan)]);
    Ok(())
}

/// __Chan_writer__ chan ,(writer)
pub fn chan_writer(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_writer__";
    let mut a = take_args(vp, name, 1)?;
    let chan_v = a.pop().unwrap();
    let chan = as_dict(name, "channel", &chan_v)?;
    let head = new_ref(m, Value::Null);
    let consumed = new_ref(m, Value::Number(0.0));
    let chain = new_dict(m);
    put(&chain, "head", head.clone())?;
    put(&chain, "consumed", consumed.clone())?;
    match get(name, "channel", &chan, "chains")? {
        Value::MSeq(s) => s.items_mut().push(Value::Dict(chain.clone())),
        other => return err(format!("{}: expected an MSeq as the chains, got {}", name, value_type_name(&other))),
    }
    let writer = new_dict(m);
    put(&writer, "chan", chan_v)?;
    put(&writer, "chain", Value::Dict(chain))?;
    put(&writer, "tail", head)?;
    put(&writer, "produced", Value::Number(0.0))?;
    put(&writer, "consumed", consumed)?;
    for key in ["cancelled", "wake", "cap", "state"] {
        let v = get(name, "channel", &chan, key)?;
        put(&writer, key, v)?;
    }
    vp.prim_return(vec![Value::Dict(writer)]);
    Ok(())
}

/// __Chan_of__ writer ,(chan)
pub fn chan_of(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_of__";
    let mut a = take_args(vp, name, 1)?;
    let writer = as_dict(name, "writer", &a.pop().unwrap())?;
    let chan = get(name, "writer", &writer, "chan")?;
    vp.prim_return(vec![chan]);
    Ok(())
}

/// __Chan_put__ onCancelled writer item ,()
pub fn chan_put(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_put__";
    check_args(vp, name, 3)?;
    let writer = match as_dict(name, "writer", &vp.olist[1]) {
        Ok(w) => w,
        Err(e) => {
            vp.olist.drain(..3);
            return Err(e);
        }
    };
    match put_or_wait(m, vp, &writer) {
        Ok(()) => Ok(()),
        Err(e) => {
            vp.olist.drain(..3);
            Err(e)
        }
    }
}

fn put_or_wait(m: &mut Machine, vp: &mut VProc, writer: &DictObj) -> PResult {
    let name = "__Chan_put__";
    let what = "writer";
    let state = get(name, what, writer, "state")?;
    // Woken up after waiting: the definition sets the state to "running" before it looks again
    if writer.find_str("waiting").is_some() && !get(name, what, writer, "waiting")?.is_null() {
        put(writer, "waiting", Value::Null)?;
        if let Value::Ref(s) = &state {
            set(m, s, string_value("running"));
        }
    }
    let consumed_ref = as_ref(name, "consumed", get(name, what, writer, "consumed")?)?;
    let consumed = as_number(name, "consumed", &consumed_ref.value.borrow())?;
    let cancelled_ref = as_ref(name, "cancelled", get(name, what, writer, "cancelled")?)?;
    let cancelled = boolean_choice(&cancelled_ref.value.borrow()) == Some(true);
    if cancelled {
        // onCancelled with the LCont of __Chan_put__, without writing
        let on_cancelled = std::mem::replace(&mut vp.olist[0], Value::Null);
        vp.olist.clear();
        vp.operator = on_cancelled;
        return Ok(());
    }
    let produced = as_number(name, "produced", &get(name, what, writer, "produced")?)?;
    let cap = as_number(name, "cap", &get(name, what, writer, "cap")?)?;
    if produced - consumed - cap < 0.0 {
        let item = std::mem::replace(&mut vp.olist[2], Value::Null);
        let tail = as_ref(name, "tail", get(name, what, writer, "tail")?)?;
        let next = new_ref(m, Value::Null);
        set(m, &tail, Value::LList(Rc::new(LList { head: item, tail: next.clone() })));
        put(writer, "tail", next)?;
        put(writer, "produced", Value::Number(produced + 1.0))?;
        let w = as_ref(name, "wake", get(name, what, writer, "wake")?)?;
        wake(m, &w);
        vp.prim_return0();
        return Ok(());
    }
    // Full: wait until the reader takes an item (or cancels), and apply the primitive again
    if let Value::Ref(s) = &state {
        set(m, s, string_value("blocked"));
    }
    put(writer, "waiting", m.true_closure())?;
    let cur = m.current.clone().expect("no current VProc");
    m.wait_change(&cur, &consumed_ref, Value::Number(consumed));
    Ok(())
}

/// __Chan_close__ writer ,()
pub fn chan_close(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_close__";
    let mut a = take_args(vp, name, 1)?;
    let writer = as_dict(name, "writer", &a.pop().unwrap())?;
    let tail = as_ref(name, "tail", get(name, "writer", &writer, "tail")?)?;
    let open = tail.value.borrow().is_null();
    if open {
        set(m, &tail, string_value("end"));
        let w = as_ref(name, "wake", get(name, "writer", &writer, "wake")?)?;
        wake(m, &w);
    }
    vp.prim_return(vec![]);
    Ok(())
}

/// __Chan_cancel__ chan ,()
pub fn chan_cancel(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_cancel__";
    let mut a = take_args(vp, name, 1)?;
    let chan = as_dict(name, "channel", &a.pop().unwrap())?;
    let cancelled = as_ref(name, "cancelled", get(name, "channel", &chan, "cancelled")?)?;
    let t = m.true_closure();
    set(m, &cancelled, t);
    let chains = match get(name, "channel", &chan, "chains")? {
        Value::MSeq(s) => s.to_vec(),
        other => return err(format!("{}: expected an MSeq as the chains, got {}", name, value_type_name(&other))),
    };
    for chain in chains {
        let chain = as_dict(name, "chain", &chain)?;
        // -1, which is never a count, wakes the writers waiting for consumed to change
        let consumed = as_ref(name, "consumed", get(name, "chain", &chain, "consumed")?)?;
        set(m, &consumed, Value::Number(-1.0));
    }
    vp.prim_return(vec![]);
    Ok(())
}

/// __Chan_reader__ chan ,(reader)
pub fn chan_reader(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_reader__";
    let mut a = take_args(vp, name, 1)?;
    let chan_v = a.pop().unwrap();
    let chan = as_dict(name, "channel", &chan_v)?;
    let reader = new_dict(m);
    put(&reader, "chan", chan_v)?;
    put(&reader, "wake", get(name, "channel", &chan, "wake")?)?;
    put(&reader, "seen", Value::Number(0.0))?;
    put(&reader, "cursors", Value::Null)?;
    put(&reader, "untilRegister", Value::Number(0.0))?;
    vp.prim_return(vec![Value::Dict(reader)]);
    Ok(())
}

/// __Chan_take__ reader ,(item found)
pub fn chan_take(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Chan_take__";
    check_args(vp, name, 1)?;
    let reader = match as_dict(name, "reader", &vp.olist[0]) {
        Ok(r) => r,
        Err(e) => {
            vp.olist.drain(..1);
            return Err(e);
        }
    };
    match take_or_wait(m, vp, &reader) {
        Ok(()) => Ok(()),
        Err(e) => {
            vp.olist.drain(..1);
            Err(e)
        }
    }
}

fn take_or_wait(m: &mut Machine, vp: &mut VProc, reader: &DictObj) -> PResult {
    let name = "__Chan_take__";
    loop {
        let w = as_ref(name, "wake", get(name, "reader", reader, "wake")?)?;
        let token = w.value.borrow().clone();
        let until = as_number(name, "untilRegister", &get(name, "reader", reader, "untilRegister")?)?;
        if until == 0.0 {
            register(m, reader)?;
            put(reader, "untilRegister", Value::Number(REGISTER_EVERY))?;
        }
        if let Some(item) = take_from(m, reader)? {
            return found(m, vp, reader, item);
        }
        // New chains may have items; if there are none, the known chains were just looked at, so wait
        // (if an item was written since, the token has changed and the VProc does not wait)
        if register(m, reader)? {
            if let Some(item) = take_from(m, reader)? {
                return found(m, vp, reader, item);
            }
        }
        if get(name, "reader", reader, "cursors")?.is_null() {
            // A writer may have added a chain while the known ones were being closed: look once more,
            // and end if there is still no open chain (none is added after that)
            register(m, reader)?;
            if get(name, "reader", reader, "cursors")?.is_null() {
                vp.olist.clear();
                let f = m.false_closure();
                vp.prim_return(vec![Value::Null, f]);
                return Ok(());
            }
            continue;
        }
        let cur = m.current.clone().expect("no current VProc");
        m.wait_change(&cur, &w, token);
        return Ok(());
    }
}

fn found(m: &mut Machine, vp: &mut VProc, reader: &DictObj, item: Value) -> PResult {
    let until = as_number("__Chan_take__", "untilRegister", &get("__Chan_take__", "reader", reader, "untilRegister")?)?;
    put(reader, "untilRegister", Value::Number(until - 1.0))?;
    vp.olist.clear();
    let t = m.true_closure();
    vp.prim_return(vec![item, t]);
    Ok(())
}

/// Add the cursors of the chains not seen yet at the end of "cursors". Whether any were added.
fn register(m: &mut Machine, reader: &DictObj) -> Result<bool, VmError> {
    let name = "__Chan_take__";
    let chan = as_dict(name, "channel", &get(name, "reader", reader, "chan")?)?;
    let chains = match get(name, "channel", &chan, "chains")? {
        Value::MSeq(s) => s.to_vec(),
        other => return err(format!("{}: expected an MSeq as the chains, got {}", name, value_type_name(&other))),
    };
    let seen = as_number(name, "seen", &get(name, "reader", reader, "seen")?)? as usize;
    if chains.len() <= seen {
        return Ok(false);
    }
    let mut cursors = list_items(name, get(name, "reader", reader, "cursors")?)?;
    for chain in &chains[seen..] {
        let c = as_dict(name, "chain", chain)?;
        let cursor = new_dict(m);
        put(&cursor, "chain", chain.clone())?;
        put(&cursor, "node", get(name, "chain", &c, "head")?)?;
        put(&cursor, "consumed", get(name, "chain", &c, "consumed")?)?;
        cursors.push(Value::Dict(cursor));
    }
    put(reader, "seen", Value::Number(chains.len() as f64))?;
    put(reader, "cursors", list_from(cursors))?;
    Ok(true)
}

/// Take an item from the first known chain that has one, dropping the closed chains before it, and
/// move that chain to the end (unless it is the only one left).
fn take_from(m: &mut Machine, reader: &DictObj) -> Result<Option<Value>, VmError> {
    let name = "__Chan_take__";
    let cursors = list_items(name, get(name, "reader", reader, "cursors")?)?;
    let mut kept = Vec::new();
    for (i, cursor) in cursors.iter().enumerate() {
        let c = as_dict(name, "cursor", cursor)?;
        let node = as_ref(name, "node", get(name, "cursor", &c, "node")?)?;
        let value = node.value.borrow().clone();
        match value {
            Value::Null => kept.push(cursor.clone()),
            Value::Str(_) | Value::StrNode(_) => {} // "end": the chain is closed
            Value::LList(cell) => {
                put(&c, "node", cell.tail.clone())?;
                let consumed = as_ref(name, "consumed", get(name, "cursor", &c, "consumed")?)?;
                let n = as_number(name, "consumed", &consumed.value.borrow())?;
                set(m, &consumed, Value::Number(n + 1.0));
                kept.extend(cursors[i + 1..].iter().cloned());
                kept.push(cursor.clone());
                put(reader, "cursors", list_from(kept))?;
                return Ok(Some(cell.head.clone()));
            }
            other => return err(format!("{}: expected a node of a chain, got {}", name, value_type_name(&other))),
        }
    }
    put(reader, "cursors", list_from(kept))?;
    Ok(None)
}
