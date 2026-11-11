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

pub fn eval(c: &CExpr, lvenv: &LVEnv, gvenv: &Rc<GVEnv>) -> Result<Value, VmError> {
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

/// Load a Body into the VProc, or run its transpiled code (vm/conma2rust), which
/// loads a Body later in the same way or leaves the VProc as a step would.
fn load_body(m: &mut Machine, vp: &mut VProc, body: &BodyDef, lvenv: LVEnv, gvenv: &Rc<GVEnv>) -> PResult {
    if m.transpiled_on {
        if let Some(id) = body.func_id {
            let table = gvenv.transpiled.borrow().clone();
            if let Some(t) = table {
                if t.fns.get(id).copied().flatten().is_some() {
                    return crate::transpiled::run(m, vp, id, lvenv, gvenv, &t);
                }
            }
        }
    }
    if body.if_site != 0 && m.fast_steps >= 3 {
        if let Some(r) = load_if(m, vp, body, &lvenv, gvenv) {
            return r;
        }
    }
    load_body_interp(vp, body, lvenv, gvenv)
}

/// Load a Body that applies __if_then_else__, __if__ or __unless__ (BodyDef::if_site)
/// and do the next two steps with it, counting them (docs/FAST-PLAN-ja.md 4.3): when its
/// Operator is that primitive and its condition a Boolean, the primitive would make the
/// chosen clause the Operator (one step), and applying that Closure without parameters
/// would push the LCont to the CChain and load its Body (another step). Here the clause
/// is loaded without making the Closures of the clauses. When __if__ or __unless__
/// chooses the clause that does nothing, the primitive returns to the LCont (one step).
/// None, without changing anything, when the Body is to be loaded as it is.
fn load_if(m: &mut Machine, vp: &mut VProc, body: &BodyDef, lvenv: &LVEnv, gvenv: &Rc<GVEnv>) -> Option<PResult> {
    let name = match body.if_site {
        crate::module::IF_THEN_ELSE => "__if_then_else__",
        crate::module::IF => "__if__",
        _ => "__unless__",
    };
    match eval(body.operator.as_ref()?, lvenv, gvenv) {
        Ok(Value::Prim(p)) if p.name == name => {}
        _ => return None,
    }
    let b = crate::prim::control::boolean_choice(&eval(&body.olist[0], lvenv, gvenv).ok()?)?;
    let clause = match (body.if_site, b) {
        (crate::module::IF_THEN_ELSE, true) | (crate::module::IF, true) => &body.olist[1],
        (crate::module::IF_THEN_ELSE, false) => &body.olist[2],
        (crate::module::UNLESS, false) => &body.olist[1],
        _ => {
            // The clause that does nothing: return to the LCont without values.
            let lcont = match &body.lcont {
                None => Value::Null,
                Some(CLCont::F(f)) => make_closure(f, lvenv, Some(gvenv)),
                Some(CLCont::V(_)) => return None,
            };
            vp.sinfo = body.sinfo.clone();
            vp.dict_site = None;
            vp.operator = lcont;
            vp.olist.clear();
            vp.lcont = None;
            vp.lvenv = lvenv.clone();
            m.steps += 1;
            m.fast_steps -= 1;
            return Some(Ok(()));
        }
    };
    let f = match clause {
        CExpr::F(f) => f,
        _ => return None,
    };
    let lcont = match &body.lcont {
        None => None,
        Some(CLCont::F(lf)) => Some(make_closure(lf, lvenv, Some(gvenv))),
        Some(CLCont::V(_)) => return None,
    };
    vp.sinfo = body.sinfo.clone();
    if let Some(lc) = lcont {
        vp.cchain_push(lc, body.sinfo.clone());
    }
    vp.olist.clear();
    vp.lcont = None;
    m.steps += 2;
    m.fast_steps -= 2;
    Some(load_body(m, vp, f.body.as_ref().unwrap(), lvenv.clone(), gvenv))
}

/// Load a Body into the VProc: its SInfo, Operator, OList and LCont,
/// evaluated in lvenv.
pub fn load_body_interp(vp: &mut VProc, body: &BodyDef, lvenv: LVEnv, gvenv: &Rc<GVEnv>) -> PResult {
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
    vp.dict_site = body.dict_site.clone();
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
            let kc = k.cchain.borrow().clone();
            match crate::vproc::cchain_split(&kc) {
                None => {
                    vp.cchain = kc;
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
        Value::Closure(c) => apply_closure(m, vp, c),
        other => {
            let shown = sexp::to_string(&crate::trace::value_to_sexp(&other));
            vp.operator = other;
            err(format!("Not a function: {}", shown))
        }
    }
}

fn apply_closure(m: &mut Machine, vp: &mut VProc, mut c: Rc<Closure>) -> PResult {
    // __LazyList_force__ of include/util.se applied to a LazyList that has been evaluated
    // (module.rs, mark_fast): pass its cell to the LCont at once, counting the 16 steps of the
    // definition (docs/FAST-PLAN-ja.md 4.2). Anything else runs the definition.
    if m.fast_steps >= crate::module::LAZY_FORCE_STEPS && vp.olist.len() == 1 && vp.lcont.is_some() && c.skip == 0 {
        if let Some(b) = &c.body {
            if b.fast.get() == crate::module::FAST_LAZY_FORCE {
                if let Value::Ref(r) = &vp.olist[0] {
                    let v = r.value.borrow();
                    if v.is_null() || matches!(*v, Value::LList(_)) {
                        let cell = v.clone();
                        drop(v);
                        vp.olist[0] = cell;
                        vp.operator = vp.lcont.take().unwrap();
                        m.steps += crate::module::LAZY_FORCE_STEPS - 1;
                        m.fast_steps -= crate::module::LAZY_FORCE_STEPS - 1;
                        return Ok(());
                    }
                }
            }
        }
    }
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
        return load_body(m, vp, &body, new_lvenv, &gvenv);
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
        return load_body(m, vp, &body, c.lvenv.clone(), &gvenv);
    }

    // As many operands as parameters, when the scheduler allows it (Machine::fast_steps):
    // bind them and load the Body in this step, as the next step would do with the
    // Closure the binding makes (Head empty, OList empty, below), and count the binding
    // as a step of its own, so that the steps are numbered as before.
    if m.fast_steps >= 2 && vp.olist.len() == params.len() {
        let mut env = c.lvenv.clone();
        for (k, v) in vp.olist.drain(..).enumerate() {
            env = lv_push(&env, params[k].clone(), v);
        }
        m.steps += 1;
        m.fast_steps -= 1;
        if let Some(lc) = vp.lcont.take() {
            let cur_sinfo = vp.sinfo.clone();
            vp.cchain_push(lc, cur_sinfo);
        }
        let body = match &c.body {
            Some(b) => b.clone(),
            None => return err("Closure has no Body"),
        };
        return load_body(m, vp, &body, env, &gvenv);
    }

    // Head not empty, OList not empty: bind as many parameters as there are operands.
    if !vp.olist.is_empty() {
        let n = params.len().min(vp.olist.len());
        // A Closure that nothing else holds (an LCont just made by load_body, as a rule) is the
        // partially applied Closure itself: bind the operands into its LVEnv and move its skip on,
        // instead of making a new Closure. No one can see the difference.
        if let Some(cm) = Rc::get_mut(&mut c) {
            let all = cm.params.clone();
            let base = cm.skip;
            let mut env = cm.lvenv.take();
            for (k, v) in vp.olist.drain(..n).enumerate() {
                env = Some(Rc::new(LVNode { name: all[base + k].clone(), value: v, next: env }));
            }
            cm.lvenv = env;
            cm.skip = base + n;
            vp.operator = Value::Closure(c);
            return Ok(());
        }
        let params = c.params();
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
