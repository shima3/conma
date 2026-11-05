//! The machine: the VProcs and the round-robin scheduler (class Scheduler of
//! test/vproc_step.py). Each scheduling step runs one Operator application of
//! the VProc at the front of the ready queue.

use crate::module::{compile_body, GVEnv};
use crate::prim::{os::write_std, VmError};
use crate::sexp::{self, Sexp};
use crate::step::step;
use crate::trace::vproc_to_sexp;
use crate::value::*;
use crate::vproc::{State, VProc};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

pub type VProcRef = Rc<RefCell<VProc>>;

#[derive(Default)]
pub struct Serials {
    pub refs: usize,
    pub mseqs: usize,
    pub dicts: usize,
    pub conts: usize,
}

pub struct Machine {
    pub vprocs: Vec<VProcRef>,
    pub ready: VecDeque<VProcRef>,
    pub steps: u64,
    pub trace: bool,
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
            steps: 0,
            trace,
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

    pub fn make_vproc(&mut self, operator: Value, olist: Vec<Value>, source_file: &str, gvenv: Rc<GVEnv>) -> VProcRef {
        self.next_serial += 1;
        Rc::new(RefCell::new(VProc::new(self.next_serial, operator, olist, source_file, gvenv)))
    }

    pub fn add(&mut self, vp: VProcRef) {
        vp.borrow_mut().state = State::Ready;
        self.vprocs.push(vp.clone());
        self.ready.push_back(vp);
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
    /// primitive exits the interpreter.
    pub fn run(&mut self) -> Option<i32> {
        loop {
            while let Some(front) = self.ready.front() {
                if !front.borrow().is_terminal() {
                    break;
                }
                let vp = self.ready.pop_front().unwrap();
                vp.borrow_mut().state = State::Finished;
            }
            if self.ready.is_empty() {
                let blocked: Vec<String> = self
                    .vprocs
                    .iter()
                    .filter(|v| v.borrow().state == State::Blocked)
                    .map(|v| v.borrow().serial.to_string())
                    .collect();
                if blocked.is_empty() {
                    return None;
                }
                write_std(2, &format!("Deadlock: no VProc is ready; blocked VProc(s): {}\n", blocked.join(", ")));
                return Some(2);
            }
            let vpr = self.ready.pop_front().unwrap();
            let mut vp = vpr.borrow_mut();
            vp.state = State::Running;
            match step(self, &mut vp) {
                Ok(()) => {
                    self.steps += 1;
                    if self.trace {
                        self.print_trace(&vp, self.steps);
                    }
                }
                Err(VmError::Error(e)) => {
                    write_std(2, &format!("Error in VProc {} at step {}: {}\n", vp.serial, self.steps, e));
                    self.steps += 1;
                    vp.state = State::Finished;
                }
                Err(VmError::Exit(code)) => return Some(code),
            }
            if vp.state == State::Running {
                if vp.is_terminal() {
                    vp.state = State::Finished;
                } else {
                    vp.state = State::Ready;
                    drop(vp);
                    self.ready.push_back(vpr);
                }
            }
        }
    }
}
