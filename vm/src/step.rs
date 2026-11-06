//! One step of a VProc: one Operator application (step in test/vproc_step.py).

use crate::module::{make_closure, BodyDef, CExpr, CLCont, GVEnv};
use crate::prim::{err, PResult, VmError};
use crate::sched::Machine;
use crate::sexp;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// Prefix msg with the source location `file:line:column` of sinfo, if any.
pub fn format_error(msg: &str, sinfo: &Option<SInfo>) -> String {
    let parts: Vec<&str> = match sinfo {
        Some(s) => s.iter().filter_map(|x| x.as_atom()).map(unquote).collect(),
        None => Vec::new(),
    };
    if parts.is_empty() {
        msg.to_string()
    } else {
        format!("{}: {}", parts[..parts.len().min(3)].join(":"), msg)
    }
}

fn eval(c: &CExpr, lvenv: &LVEnv, gvenv: &Rc<GVEnv>) -> Result<Value, VmError> {
    match c {
        CExpr::L(i) => match lv_get(lvenv, *i) {
            Some(v) => Ok(v.clone()),
            None => err(format!("LVEnv[{}] out of range", i)),
        },
        CExpr::C(v) => Ok(v.clone()),
        CExpr::G(g, name) => match gvenv.get(*g) {
            Some(v) => Ok(v),
            None => err(format!("G{} ({}) Undefined", g, py_repr(unquote(name)))),
        },
        CExpr::F(f) => Ok(make_closure(f, lvenv, Some(gvenv))),
        CExpr::X(x) => match x.as_list() {
            Some(l) if !l.is_empty() => match l[0].as_atom() {
                Some("Variable") => err(format!(
                    "Unknown binding: {}",
                    l.get(3).map(sexp::to_string).unwrap_or_else(|| "None".into())
                )),
                Some(k) => err(format!("Unknown expr kind: {}", py_repr(k))),
                None => err(format!("Cannot eval: {}", sexp::to_string(x))),
            },
            _ => err(format!("Cannot eval: {}", sexp::to_string(x))),
        },
    }
}

/// Load a Body into the VProc: its SInfo, Operator, OList and LCont,
/// evaluated in lvenv.
fn load_body(vp: &mut VProc, body: &BodyDef, lvenv: LVEnv, gvenv: &Rc<GVEnv>) -> PResult {
    let operator = match &body.operator {
        Some(c) => eval(c, &lvenv, gvenv)?,
        None => return err("Primitive not implemented: None"),
    };
    // Reuse the Vec of the old OList, so that a step does not allocate one.
    let mut olist = std::mem::take(&mut vp.olist);
    olist.clear();
    for c in &body.olist {
        match eval(c, &lvenv, gvenv) {
            Ok(v) => olist.push(v),
            Err(e) => {
                olist.clear();
                vp.olist = olist;
                return Err(e);
            }
        }
    }
    let lcont = match &body.lcont {
        None => None,
        Some(CLCont::F(f)) => Some(make_closure(f, &lvenv, Some(gvenv))),
        Some(CLCont::V(_)) => return err("an LCont written as a __Closure__ value is not supported"),
    };
    vp.sinfo = body.sinfo.clone();
    vp.operator = operator;
    vp.olist = olist;
    vp.lcont = lcont;
    vp.lvenv = lvenv;
    Ok(())
}

pub fn step(m: &mut Machine, vp: &mut VProc) -> PResult {
    let operator = std::mem::replace(&mut vp.operator, Value::Null);

    // Null Operator
    if operator.is_null() {
        match vp.lcont.take() {
            Some(lc) => vp.operator = lc,
            None => match vp.cchain_pop() {
                Some(cf) => {
                    vp.operator = cf.closure.clone();
                    vp.sinfo = cf.sinfo.clone();
                }
                None => {
                    vp.operator = operator;
                    return err("Null Operator: CChain empty, no LCont");
                }
            },
        }
        return Ok(());
    }

    match operator {
        Value::Seq(seq) => {
            if !vp.olist.is_empty() {
                vp.operator = vp.olist.swap_remove(0);
                vp.olist = seq.olist.clone();
                vp.lvenv = seq.lvenv.clone();
            } else {
                match vp.cchain_pop() {
                    Some(cf) => {
                        vp.olist = seq.olist.clone();
                        vp.operator = cf.closure.clone();
                        vp.sinfo = cf.sinfo.clone();
                        vp.lvenv = seq.lvenv.clone();
                    }
                    None => {
                        vp.operator = Value::Seq(seq);
                        return err("SeqClosure: no OList and CChain empty");
                    }
                }
            }
            Ok(())
        }
        Value::Prim(p) => {
            vp.operator = Value::Prim(p);
            (p.f)(m, vp)
        }
        Value::Cont(k) => {
            // Set the CChain back and return to the caller's LCont with all the operands.
            vp.lcont = None;
            match crate::vproc::cchain_split(&k.cchain) {
                None => {
                    vp.cchain = k.cchain.clone();
                    vp.operator = Value::Null;
                    vp.olist = Vec::new();
                }
                Some((cf, rest)) => {
                    vp.cchain = rest;
                    vp.operator = cf.closure.clone();
                    vp.sinfo = cf.sinfo.clone();
                }
            }
            Ok(())
        }
        Value::Closure(c) => apply_closure(vp, c),
        other => {
            let shown = sexp::to_string(&crate::trace::value_to_sexp(&other));
            vp.operator = other;
            err(format!("Not a function: {}", shown))
        }
    }
}

fn apply_closure(vp: &mut VProc, c: Rc<Closure>) -> PResult {
    let gvenv = c.gvenv.clone().unwrap_or_else(|| vp.gvenv.clone());

    // Variadic: the parameter is bound to a Seq of all the operands.
    if c.variadic {
        let cur_sinfo = vp.sinfo.clone();
        if let Some(lc) = vp.lcont.take() {
            vp.cchain_push(lc, cur_sinfo.clone());
        }
        let olist = std::mem::take(&mut vp.olist);
        let seq = Value::Seq(Rc::new(SeqClosure { olist, sinfo: cur_sinfo, lvenv: vp.lvenv.clone() }));
        let new_lvenv = lv_push(&c.lvenv, c.params()[0].clone(), seq);
        let body = match &c.body {
            Some(b) => b.clone(),
            None => return err("Closure has no Body"),
        };
        return load_body(vp, &body, new_lvenv, &gvenv);
    }

    let params = c.params();

    // Head empty
    if params.is_empty() {
        let cur_sinfo = vp.sinfo.clone();
        if let Some(lc) = vp.lcont.take() {
            vp.cchain_push(lc, cur_sinfo.clone());
        }
        if !vp.olist.is_empty() {
            let olist = std::mem::take(&mut vp.olist);
            let seq = Value::Seq(Rc::new(SeqClosure { olist, sinfo: cur_sinfo.clone(), lvenv: vp.lvenv.clone() }));
            vp.cchain_push(seq, cur_sinfo);
        }
        let body = match &c.body {
            Some(b) => b.clone(),
            None => return err("Closure has no Body"),
        };
        return load_body(vp, &body, c.lvenv.clone(), &gvenv);
    }

    // Head not empty, OList not empty: bind as many parameters as there are operands.
    if !vp.olist.is_empty() {
        let n = params.len().min(vp.olist.len());
        let mut env = c.lvenv.clone();
        for (k, v) in vp.olist.drain(..n).enumerate() {
            env = lv_push(&env, params[k].clone(), v);
        }
        vp.operator = Value::Closure(Rc::new(Closure {
            params: c.params.clone(),
            skip: c.skip + n,
            body: c.body.clone(),
            lvenv: env,
            gvenv: c.gvenv.clone(),
            variadic: false,
        }));
        return Ok(());
    }

    // Head not empty, OList empty: pass the partially applied Closure on.
    match vp.lcont.take() {
        Some(lc) => {
            vp.olist = vec![Value::Closure(c)];
            vp.operator = lc;
        }
        None => match vp.cchain_pop() {
            Some(cf) => {
                vp.olist = vec![Value::Closure(c)];
                vp.operator = cf.closure.clone();
                vp.sinfo = cf.sinfo.clone();
            }
            None => {
                vp.operator = Value::Closure(c);
                return err("Partial apply: OList empty, no LCont, CChain empty");
            }
        },
    }
    Ok(())
}
