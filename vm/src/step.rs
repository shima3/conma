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
        CExpr::S(k) => match lv_slot(lvenv, *k) {
            Some(v) => Ok(v.clone()),
            None => err(format!("frame slot {} out of range", k)),
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
fn load_body(m: &mut Machine, vp: &mut VProc, body: &BodyDef, mut lvenv: LVEnv, gvenv: &Rc<GVEnv>) -> PResult {
    if m.transpiled_on {
        if let Some(id) = body.func_id {
            let table = gvenv.transpiled.borrow().clone();
            if let Some(t) = table {
                if t.fns.get(id).copied().flatten().is_some() {
                    // the LVEnvs that transpiled code leaves in vp have no scope kept
                    vp.scope = None;
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
    if body.fold_site && m.fast_steps >= 4 && m.fold_constants {
        if let Some(r) = load_const(m, vp, body, &lvenv, gvenv) {
            return r;
        }
    }
    if body.prim_site && m.fast_steps >= 4 {
        if let Some(r) = load_prim(m, vp, body, &mut lvenv, gvenv) {
            return r;
        }
    }
    load_body_interp(vp, body, lvenv, gvenv)
}

/// Load a Body `__Number_fromString__ onError "text" ,(x) ...` (BodyDef::fold_site) whose
/// Operator is the primitive: the three steps of the primitive, binding x and loading the Body
/// of the LCont are counted, and the Body of the LCont is loaded with the Number in place of x
/// (module::const_fold), without parsing the text or binding x. None when the Body is to be
/// loaded as it is.
fn load_const(m: &mut Machine, vp: &mut VProc, body: &BodyDef, lvenv: &LVEnv, gvenv: &Rc<GVEnv>) -> Option<PResult> {
    match eval(body.operator.as_ref()?, lvenv, gvenv) {
        Ok(Value::Prim(p)) if p.name == "__Number_fromString__" => {}
        _ => return None,
    }
    // The onError, read as when the Body is loaded as it is: a global without a value stops the
    // VProc there (loaded as it is). A local or a function expression is always read.
    if let Some(e @ crate::module::CExpr::G(..)) = body.olist.first() {
        if eval(e, lvenv, gvenv).is_err() {
            return None;
        }
    }
    let folded = crate::module::const_fold(body)?;
    vp.sinfo = body.sinfo.clone();
    m.steps += 3;
    m.fast_steps -= 3;
    Some(load_body(m, vp, &folded, lvenv.clone(), gvenv))
}

/// The scope of the LVEnv of vp (VProc::scope), for a SeqClosure that keeps that LVEnv: set
/// wherever the VM loads a Body and sets the LVEnv of vp, so that a printed SeqClosure names its
/// variables as the Python VM does, with or without --trace; none after transpiled code.
fn kept_scope(vp: &VProc) -> crate::module::Scope {
    vp.scope.clone()
}

/// True if v is Machine::inline_mark.
fn is_mark(m: &Machine, v: &Value) -> bool {
    matches!((v, &m.inline_mark), (Value::Closure(a), Value::Closure(b)) if Rc::ptr_eq(a, b))
}

/// Load a Body that applies one of INLINE_PRIMS with an LCont written as a function
/// (BodyDef::prim_site), run the primitive at once (the next step), and if it passes as
/// many values as the LCont has parameters, apply the LCont too (binding them and loading
/// its Body, the steps after), counting each step (docs/FAST-PLAN-ja.md 4.4): the
/// Closure of the LCont is not made then. The primitive gets Machine::inline_mark as its
/// LCont; whatever else it does, the Closure of the LCont is made and put where the mark
/// is, as if the steps had been taken one by one. None, without changing anything, when
/// the Body is to be loaded as it is.
fn load_prim(m: &mut Machine, vp: &mut VProc, body: &BodyDef, lvenv: &mut LVEnv, gvenv: &Rc<GVEnv>) -> Option<PResult> {
    let p = match eval(body.operator.as_ref()?, lvenv, gvenv) {
        Ok(Value::Prim(p)) if crate::module::INLINE_PRIMS.contains(&p.name) => p,
        _ => return None,
    };
    let f = match &body.lcont {
        Some(CLCont::F(f)) => f.clone(),
        _ => return None,
    };
    let mut olist = std::mem::take(&mut vp.olist);
    olist.clear();
    for c in &body.olist {
        match eval(c, lvenv, gvenv) {
            Ok(v) => olist.push(v),
            Err(_) => {
                olist.clear();
                vp.olist = olist;
                return None;
            }
        }
    }
    vp.sinfo = body.sinfo.clone();
    vp.dict_site = body.dict_site.clone();
    vp.operator = Value::Prim(p);
    vp.olist = olist;
    vp.lcont = Some(m.inline_mark.clone());
    vp.lvenv = lvenv.clone();
    // The step of the primitive.
    m.steps += 1;
    m.fast_steps -= 1;
    let r = (p.f)(m, vp);
    if r.is_ok() && is_mark(m, &vp.operator) && vp.olist.len() == f.params.len() {
        // The steps of applying the LCont: binding the values (if any) and loading its Body.
        let fb = f.body.as_ref().unwrap();
        let env = if fb.frame == 0 && !matches!(lvenv.as_ref().map(|n| &n.value), Some(Value::Frame(_))) {
            let mut env = lvenv.clone();
            for v in vp.olist.drain(..) {
                env = lv_push(&env, v);
            }
            env
        } else {
            // Nothing else is to hold the frame node, so that the values are added in place.
            vp.lvenv = None;
            lv_bind(std::mem::take(lvenv), vp.olist.drain(..), fb.frame, fb.frame_size)
        };
        let k = if f.params.is_empty() { 1 } else { 2 };
        m.steps += k;
        m.fast_steps -= k;
        vp.lcont = None;
        return Some(load_body(m, vp, f.body.as_ref().unwrap(), env, gvenv));
    }
    // Left to the scheduler with the LVEnv of this Body: its scope too (kept_scope; when the
    // primitive goes on to its LCont, the Body of the LCont sets it).
    vp.scope = body.scope.clone();
    if is_mark(m, &vp.operator) {
        vp.operator = make_closure(&f, lvenv, Some(gvenv));
    } else if vp.lcont.as_ref().map_or(false, |l| is_mark(m, l)) {
        vp.lcont = Some(make_closure(&f, lvenv, Some(gvenv)));
    }
    Some(r)
}

/// Load a Body that applies __if_then_else__, __if__, __unless__, __if_null_then_else__,
/// __unless_null__, __Number_if_zero_then_else__, __Number_unless_zero__, __Number_if_greater__ or
/// __Number_if_greater_then_else__ (BodyDef::if_site) and do the next two steps with it, counting them (docs/FAST-PLAN-ja.md 4.3): when its
/// Operator is that primitive and its condition a Boolean, the primitive would make the
/// chosen clause the Operator (one step; __if_null_then_else__ and __unless_null__ choose by
/// whether their first operand is __NULL__, __Number_if_zero_then_else__ and __Number_unless_zero__ by whether
/// it is the Number 0, __Number_if_greater__ and __Number_if_greater_then_else__ by comparing
/// their first two operands, and leave
/// any other value to the primitive, which stops with an error), and applying that Closure without parameters
/// would push the LCont to the CChain and load its Body (another step). Here the clause
/// is loaded without making the Closures of the clauses. When __if__ or __unless__
/// chooses the clause that does nothing, the primitive returns to the LCont (one step).
/// None, without changing anything, when the Body is to be loaded as it is.
fn load_if(m: &mut Machine, vp: &mut VProc, body: &BodyDef, lvenv: &LVEnv, gvenv: &Rc<GVEnv>) -> Option<PResult> {
    use crate::module::{IF, IF_GREATER, IF_GREATER_THEN_ELSE, IF_NULL_THEN_ELSE, IF_THEN_ELSE, IF_ZERO_THEN_ELSE, UNLESS, UNLESS_NULL, UNLESS_ZERO};
    let name = match body.if_site {
        IF_THEN_ELSE => "__if_then_else__",
        IF => "__if__",
        UNLESS => "__unless__",
        IF_NULL_THEN_ELSE => "__if_null_then_else__",
        UNLESS_NULL => "__unless_null__",
        IF_ZERO_THEN_ELSE => "__Number_if_zero_then_else__",
        UNLESS_ZERO => "__Number_unless_zero__",
        IF_GREATER => "__Number_if_greater__",
        _ => "__Number_if_greater_then_else__",
    };
    match eval(body.operator.as_ref()?, lvenv, gvenv) {
        Ok(Value::Prim(p)) if p.name == name => {}
        _ => return None,
    }
    let first = eval(&body.olist[0], lvenv, gvenv).ok()?;
    let b = match body.if_site {
        IF_NULL_THEN_ELSE | UNLESS_NULL => first.is_null(),
        IF_ZERO_THEN_ELSE | UNLESS_ZERO => match first {
            Value::Number(x) => x == 0.0,
            _ => return None,
        },
        IF_GREATER | IF_GREATER_THEN_ELSE => match (first, eval(&body.olist[1], lvenv, gvenv).ok()?) {
            (Value::Number(a), Value::Number(b)) => a > b,
            _ => return None,
        },
        _ => crate::prim::control::boolean_choice(&first)?,
    };
    let clause = match (body.if_site, b) {
        (IF_THEN_ELSE | IF | IF_NULL_THEN_ELSE | IF_ZERO_THEN_ELSE, true) => &body.olist[1],
        (IF_GREATER | IF_GREATER_THEN_ELSE, true) => &body.olist[2],
        (IF_GREATER_THEN_ELSE, false) => &body.olist[3],
        (IF_THEN_ELSE | IF_NULL_THEN_ELSE | IF_ZERO_THEN_ELSE, false) => &body.olist[2],
        (UNLESS | UNLESS_NULL | UNLESS_ZERO, false) => &body.olist[1],
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
            vp.scope = body.scope.clone();
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
    vp.scope = body.scope.clone();
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
                vp.scope = seq.scope.clone();
            } else {
                match vp.cchain_pop() {
                    Some(cf) => {
                        vp.olist = seq.olist.clone();
                        vp.operator = cf.closure.clone();
                        vp.sinfo = cf.sinfo.clone();
                        vp.lvenv = seq.lvenv.clone();
                        vp.scope = seq.scope.clone();
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

/// How the values passed to c are bound (BodyDef::frame): a frame that the first values bound
/// to c made is added to.
fn frame_of(c: &Closure) -> u8 {
    match c.body.as_ref().map_or(0, |b| b.frame) {
        1 if c.skip > 0 => 2,
        f => f,
    }
}

fn frame_size(c: &Closure) -> u32 {
    c.body.as_ref().map_or(0, |b| b.frame_size)
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
        let seq = Value::Seq(Rc::new(SeqClosure { olist, sinfo: cur_sinfo, lvenv: vp.lvenv.clone(), scope: kept_scope(vp) }));
        let new_lvenv = lv_push(&c.lvenv, seq);
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
            let seq = Value::Seq(Rc::new(SeqClosure { olist, sinfo: cur_sinfo.clone(), lvenv: vp.lvenv.clone(), scope: kept_scope(vp) }));
            vp.cchain_push(seq, cur_sinfo);
        }
        let body = match &c.body {
            Some(b) => b.clone(),
            None => return err("Closure has no Body"),
        };
        // A frame (module::frame_chain) is left to the Body alone, so that it adds to it in place.
        let env = if matches!(c.lvenv.as_ref().map(|n| &n.value), Some(Value::Frame(_))) {
            vp.lvenv = None;
            match Rc::get_mut(&mut c) {
                Some(cm) => cm.lvenv.take(),
                None => c.lvenv.clone(),
            }
        } else {
            c.lvenv.clone()
        };
        return load_body(m, vp, &body, env, &gvenv);
    }

    // As many operands as parameters, when the scheduler allows it (Machine::fast_steps):
    // bind them and load the Body in this step, as the next step would do with the
    // Closure the binding makes (Head empty, OList empty, below), and count the binding
    // as a step of its own, so that the steps are numbered as before.
    if m.fast_steps >= 2 && vp.olist.len() == params.len() {
        let frame = frame_of(&c);
        let env = if frame == 0 {
            let mut env = c.lvenv.clone();
            for v in vp.olist.drain(..) {
                env = lv_push(&env, v);
            }
            env
        } else {
            vp.lvenv = None;
            let env = match Rc::get_mut(&mut c) {
                Some(cm) => cm.lvenv.take(),
                None => c.lvenv.clone(),
            };
            lv_bind(env, vp.olist.drain(..), frame, frame_size(&c))
        };
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
        let frame = frame_of(&c);
        let size = frame_size(&c);
        if let Some(cm) = Rc::get_mut(&mut c) {
            let base = cm.skip;
            let env = cm.lvenv.take();
            cm.lvenv = lv_bind(env, vp.olist.drain(..n), frame, size);
            cm.skip = base + n;
            vp.operator = Value::Closure(c);
            return Ok(());
        }
        let env = lv_bind(c.lvenv.clone(), vp.olist.drain(..n), frame, size);
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
