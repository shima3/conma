//! CChain and CFrame primitives, and __Cont_get__.

use super::{err, take_args, PResult};
use crate::sched::Machine;
use crate::step::format_error;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

pub fn get(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let cc = vp.cchain.clone();
    vp.prim_return(vec![cc]);
    Ok(())
}

/// Replace the CChain and invoke the LCont directly.
pub fn set(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__CChain_set__", 1)?;
    let cc = a.pop().unwrap();
    if !(cc.is_null() || matches!(cc, Value::LList(_))) {
        return err(format!(
            "__CChain_set__: expected CChain (LList or null), got {}",
            value_type_name(&cc)
        ));
    }
    vp.cchain = cc;
    vp.operator = vp.lcont.take().unwrap_or(Value::Null);
    vp.olist = Vec::new();
    Ok(())
}

pub fn push_cframe(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__CChain_push_CFrame__", 1)?;
    let cf = a.pop().unwrap();
    if !matches!(cf, Value::CFrame(_)) {
        return err(format!("__CChain_push_CFrame__: expected CFrame, got {}", value_type_name(&cf)));
    }
    let tail = std::mem::replace(&mut vp.cchain, Value::Null);
    vp.cchain = Value::LList(Rc::new(LList { head: cf, tail }));
    vp.prim_return(vec![]);
    Ok(())
}

pub fn pop_cframe(_m: &mut Machine, vp: &mut VProc) -> PResult {
    match vp.cchain_pop() {
        None => vp.prim_return(vec![Value::Null]),
        Some(cf) => vp.prim_return(vec![Value::CFrame(cf)]),
    }
    Ok(())
}

/// __Cont_get__ ,(return): pass the escape continuation of the calling
/// function. Without an LCont, the top CFrame is the LCont.
pub fn cont_get(m: &mut Machine, vp: &mut VProc) -> PResult {
    let lc = match vp.lcont.take() {
        Some(lc) => lc,
        None => match vp.cchain_pop() {
            Some(cf) => {
                vp.sinfo = cf.sinfo.clone();
                cf.closure.clone()
            }
            None => return err(format_error("__Cont_get__: no LCont and CChain empty", &vp.sinfo)),
        },
    };
    m.serials.conts += 1;
    let k = Value::Cont(Rc::new(ContObj { serial: m.serials.conts, cchain: vp.cchain.clone() }));
    vp.operator = lc;
    vp.olist = vec![k];
    Ok(())
}

pub fn cframe_get_closure(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__CFrame_get_Closure__", 1)?;
    match &a[0] {
        Value::CFrame(cf) => {
            let c = cf.closure.clone();
            vp.prim_return(vec![c]);
            Ok(())
        }
        v => err(format!("__CFrame_get_Closure__: expected CFrame, got {}", value_type_name(v))),
    }
}

pub fn cframe_get_sinfo(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let a = take_args(vp, "__CFrame_get_SInfo__", 1)?;
    match &a[0] {
        Value::CFrame(cf) => {
            let parts: Vec<&str> = match &cf.sinfo {
                Some(s) => s.iter().filter_map(|x| x.as_atom()).map(unquote).collect(),
                None => Vec::new(),
            };
            vp.prim_return(vec![string_value(&parts.join(":"))]);
            Ok(())
        }
        v => err(format!("__CFrame_get_SInfo__: expected CFrame, got {}", value_type_name(v))),
    }
}

pub fn cframe_new(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let mut a = take_args(vp, "__CFrame_new__", 2)?;
    let sinfo = a.pop().unwrap();
    let closure = a.pop().unwrap();
    if value_type_name(&closure) != "Closure" {
        return err(format!("__CFrame_new__: expected Closure, got {}", value_type_name(&closure)));
    }
    let parts: Vec<crate::sexp::Sexp> = if sinfo.is_null() {
        Vec::new()
    } else if value_type_name(&sinfo) == "String" {
        let text = value_to_display(&sinfo).map_err(super::VmError::Error)?;
        if text.is_empty() {
            Vec::new()
        } else {
            text.split(':').map(|x| crate::sexp::Sexp::atom(&quoted(x))).collect()
        }
    } else {
        return err(format!(
            "__CFrame_new__: expected String or null SInfo, got {}",
            value_type_name(&sinfo)
        ));
    };
    vp.prim_return(vec![Value::CFrame(Rc::new(CFrame { closure, sinfo: Some(Rc::new(parts)) }))]);
    Ok(())
}
