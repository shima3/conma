//! The machine: the VProcs and the round-robin scheduler (class Scheduler of
//! test/vproc_step.py). Each scheduling step runs one Operator application of
//! the VProc at the front of the ready queue.

use crate::module::{compile_body, GVEnv};
use crate::prim::{os::write_std, VmError};
use crate::sexp::{self, Sexp};
use crate::step::step;
use crate::trace::vproc_to_sexp;
use crate::value::*;
use crate::prim::lazy::{evaluating_lazy, lazy_abandoned};
use crate::vproc::{State, VProc, VProcCell};
pub use crate::vproc::VProcRef;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Default)]
pub struct Serials {
    pub refs: usize,
    pub mseqs: usize,
    pub dicts: usize,
    pub conts: usize,
}

pub struct Machine {
    /// Every VProc, in creation order.
    pub vprocs: Vec<VProcRef>,
    pub ready: VecDeque<VProcRef>,
    /// The VProc running the current step.
    pub current: Option<VProcRef>,
    pub steps: u64,
    pub trace: bool,
    /// (VProc, VProc) waiting for another VProc to finish.
    joining: Vec<(VProcRef, VProcRef)>,
    /// (VProc, LazyList) waiting for another VProc to evaluate a LazyList.
    lazy: Vec<(VProcRef, Rc<RefObj>)>,
    /// (VProc, Ref, value) waiting for the value of a Ref to change.
    changes: Vec<(VProcRef, Rc<RefObj>, Value)>,
    /// A VProc may have finished since joining was checked.
    pub finished_any: bool,
    /// A Ref may have changed since the VProcs waiting for a change were checked.
    pub ref_changed: bool,
    /// Serial numbers of the values that traces show by number.
    pub serials: Serials,
    next_serial: usize,
    true_closure: Value,
    false_closure: Value,
}

/// The Boolean Closures `,(t f) t` and `,(t f) f`, with the AST that
/// test/vproc_step.py builds for them.
fn bool_closure(param: &str) -> Value {
    let a = Sexp::atom;
    let l = Sexp::list;
    let p0 = Sexp::pos0;
    let idx = if param == "t" { "1" } else { "0" };
    let body = l(vec![
        a("Body"),
        p0(),
        l(vec![a("Operator"), p0(), l(vec![a("Null"), p0()])]),
        l(vec![
            a("OList"),
            p0(),
            l(vec![a("Variable"), p0(), a(&format!("\"{}\"", param)), l(vec![a("L"), a(idx)])]),
        ]),
        l(vec![a("LCont"), p0()]),
    ]);
    Value::Closure(Rc::new(Closure {
        params: Rc::from(vec![Rc::from("t"), Rc::from("f")]),
        skip: 0,
        body: Some(compile_body(&body)),
        lvenv: None,
        gvenv: None,
        variadic: false,
    }))
}

impl Machine {
    pub fn new(trace: bool) -> Machine {
        Machine {
            vprocs: Vec::new(),
            ready: VecDeque::new(),
            current: None,
            steps: 0,
            trace,
            joining: Vec::new(),
            lazy: Vec::new(),
            changes: Vec::new(),
            finished_any: false,
            ref_changed: false,
            serials: Serials::default(),
            next_serial: 0,
            true_closure: bool_closure("t"),
            false_closure: bool_closure("f"),
        }
    }

    pub fn true_closure(&self) -> Value {
        self.true_closure.clone()
    }
    pub fn false_closure(&self) -> Value {
        self.false_closure.clone()
    }

    /// A VProc that applies operator to olist in the directory cwd.
    pub fn make_vproc(
        &mut self,
        operator: Value,
        olist: Vec<Value>,
        source_file: Option<&str>,
        gvenv: Rc<GVEnv>,
        cwd: String,
    ) -> VProcRef {
        self.next_serial += 1;
        let serial = self.next_serial;
        Rc::new(VProcCell {
            serial,
            state: Cell::new(State::Ready),
            kill_pending: Cell::new(false),
            vp: RefCell::new(VProc::new(serial, operator, olist, source_file, gvenv, cwd)),
        })
    }

    pub fn add(&mut self, vp: VProcRef) {
        vp.state.set(State::Ready);
        self.vprocs.push(vp.clone());
        self.ready.push_back(vp);
    }

    /// True if vp is the VProc running the current step.
    pub fn is_current(&self, vp: &VProcRef) -> bool {
        self.current.as_ref().map_or(false, |c| Rc::ptr_eq(c, vp))
    }

    fn remove_ready(&mut self, vp: &VProcRef) {
        if let Some(i) = self.ready.iter().position(|x| Rc::ptr_eq(x, vp)) {
            self.ready.remove(i);
        }
    }

    fn make_waiting(&mut self, vp: &VProcRef) {
        if vp.state.get() == State::Ready {
            self.remove_ready(vp);
        }
        vp.state.set(State::Waiting);
    }

    /// Make vp wait until the VProc target has finished.
    pub fn join(&mut self, vp: &VProcRef, target: &VProcRef) {
        self.make_waiting(vp);
        self.joining.push((vp.clone(), target.clone()));
    }

    /// Make vp wait until the LazyList is no longer being evaluated by another VProc.
    pub fn wait_lazy(&mut self, vp: &VProcRef, lazy: &Rc<RefObj>) {
        self.make_waiting(vp);
        self.lazy.push((vp.clone(), lazy.clone()));
    }

    /// Make vp wait until the value of r is no longer value.
    pub fn wait_change(&mut self, vp: &VProcRef, r: &Rc<RefObj>, value: Value) {
        self.make_waiting(vp);
        self.changes.push((vp.clone(), r.clone(), value));
    }

    fn wake(&mut self, vp: &VProcRef) {
        vp.state.set(State::Ready);
        self.ready.push_back(vp.clone());
    }

    fn check_joining(&mut self) {
        let list = std::mem::take(&mut self.joining);
        for (vp, target) in list {
            if target.state.get() == State::Finished {
                self.wake(&vp);
            } else {
                self.joining.push((vp, target));
            }
        }
    }

    fn check_lazy(&mut self) {
        let list = std::mem::take(&mut self.lazy);
        for (vp, r) in list {
            let owner = match &*r.value.borrow() {
                Value::VProc(o) => Some(o.clone()),
                _ => None,
            };
            let still = match owner {
                Some(o) if o.state.get() != State::Finished => !lazy_abandoned(&r, &o.vp.borrow().cchain),
                _ => false,
            };
            if still {
                self.lazy.push((vp, r));
            } else {
                self.wake(&vp);
            }
        }
    }

    fn check_changes(&mut self) {
        let list = std::mem::take(&mut self.changes);
        for (vp, r, value) in list {
            if same_for_wait(&r.value.borrow(), &value) {
                self.changes.push((vp, r, value));
            } else {
                self.wake(&vp);
            }
        }
    }

    /// Block vp. A blocked or finished VProc is left as it is.
    pub fn suspend(&mut self, vp: &VProcRef) {
        match vp.state.get() {
            State::Ready => {
                self.remove_ready(vp);
                vp.state.set(State::Blocked);
            }
            State::Running => vp.state.set(State::Blocked),
            _ => {}
        }
    }

    /// Make a blocked vp ready.
    pub fn resume(&mut self, vp: &VProcRef) {
        if vp.state.get() == State::Blocked {
            self.wake(vp);
        }
    }

    /// Finish target, or, if it is evaluating a LazyList that it claimed,
    /// when it has evaluated them. cur is the VProc of the current step.
    pub fn kill(&mut self, cur: &mut VProc, target: &VProcRef) {
        if target.state.get() == State::Finished {
            return;
        }
        let evaluating = if self.is_current(target) {
            evaluating_lazy(target, cur)
        } else {
            evaluating_lazy(target, &mut target.vp.borrow_mut())
        };
        if evaluating {
            target.kill_pending.set(true);
            return;
        }
        self.kill_now(target);
    }

    /// Finish vp at once, and remove it from the ready queue and every list
    /// of waiting VProcs.
    pub fn kill_now(&mut self, vp: &VProcRef) {
        if vp.state.get() == State::Finished {
            return;
        }
        if vp.state.get() == State::Ready {
            self.remove_ready(vp);
        }
        self.joining.retain(|(v, _)| !Rc::ptr_eq(v, vp));
        self.lazy.retain(|(v, _)| !Rc::ptr_eq(v, vp));
        self.changes.retain(|(v, _, _)| !Rc::ptr_eq(v, vp));
        vp.state.set(State::Finished);
        self.finished_any = true;
    }

    pub fn print_trace(&self, vp: &VProc, step_n: u64) {
        let mut label = if step_n > 0 { format!("[step {}]", step_n) } else { "[initial]".to_string() };
        if vp.serial != 1 {
            label.pop();
            label.push_str(&format!(" vproc {}]", vp.serial));
        }
        write_std(2, &format!("{} {}\n", label, sexp::to_string(&vproc_to_sexp(vp))));
    }

    /// Run until every VProc has finished. Returns the exit code when a
    /// primitive exits the interpreter, or on deadlock.
    pub fn run(&mut self) -> Option<i32> {
        loop {
            while let Some(front) = self.ready.front() {
                if !front.vp.borrow().is_terminal() {
                    break;
                }
                let vp = self.ready.pop_front().unwrap();
                vp.state.set(State::Finished);
                self.finished_any = true;
            }
            if !self.joining.is_empty() && self.finished_any {
                self.finished_any = false;
                self.check_joining();
            }
            if !self.lazy.is_empty() {
                self.check_lazy();
            }
            if !self.changes.is_empty() && self.ref_changed {
                self.ref_changed = false;
                self.check_changes();
            }
            if self.ready.is_empty() {
                let waiting: Vec<&VProcRef> = self
                    .joining
                    .iter()
                    .map(|(v, _)| v)
                    .chain(self.lazy.iter().map(|(v, _)| v))
                    .chain(self.changes.iter().map(|(v, _, _)| v))
                    .collect();
                let blocked: Vec<String> = self
                    .vprocs
                    .iter()
                    .filter(|v| v.state.get() == State::Blocked || waiting.iter().any(|w| Rc::ptr_eq(w, v)))
                    .map(|v| v.serial.to_string())
                    .collect();
                if blocked.is_empty() {
                    return None;
                }
                write_std(2, &format!("Deadlock: no VProc is ready; blocked VProc(s): {}\n", blocked.join(", ")));
                return Some(2);
            }
            let cell = self.ready.pop_front().unwrap();
            cell.state.set(State::Running);
            self.current = Some(cell.clone());
            let mut vp = cell.vp.borrow_mut();
            let r = step(self, &mut vp);
            self.current = None;
            match r {
                Ok(()) => {
                    self.steps += 1;
                    if self.trace {
                        self.print_trace(&vp, self.steps);
                    }
                }
                Err(VmError::Error(e)) => {
                    write_std(2, &format!("Error in VProc {} at step {}: {}\n", vp.serial, self.steps, e));
                    self.steps += 1;
                    cell.state.set(State::Finished);
                    self.finished_any = true;
                }
                Err(VmError::Exit(code)) => return Some(code),
            }
            if cell.kill_pending.get() && cell.state.get() != State::Finished && !evaluating_lazy(&cell, &mut vp) {
                self.kill_now(&cell);
            }
            if cell.state.get() == State::Running {
                if vp.is_terminal() {
                    cell.state.set(State::Finished);
                    self.finished_any = true;
                } else {
                    cell.state.set(State::Ready);
                    drop(vp);
                    self.ready.push_back(cell);
                }
            }
        }
    }
}

/// `ref.value is value` of Python. Numbers have no identity here, so equal
/// Numbers count as the same.
fn same_for_wait(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.to_bits() == y.to_bits(),
        _ => same(a, b),
    }
}
