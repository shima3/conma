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
    pub streams: usize,
    pub osprocs: usize,
}

pub struct Machine {
    /// Every VProc, in creation order.
    pub vprocs: Vec<VProcRef>,
    /// The length of vprocs after finished VProcs were last removed from it.
    vprocs_pruned: usize,
    pub ready: VecDeque<VProcRef>,
    /// The VProc running the current step.
    pub current: Option<VProcRef>,
    pub steps: u64,
    pub trace: bool,
    /// Run the transpiled code of modules (vm/conma2rust): not with --trace or
    /// CONMA_PROFILE, which count every step, nor with CONMA_TRANSPILED_OFF.
    pub transpiled_on: bool,
    /// Step counts for CONMA_PROFILE.
    pub profile: Option<crate::profile::Profile>,
    /// How many steps the next step may do at once, counting each of them
    /// (docs/FAST-PLAN-ja.md 4.1 and 4.2): the steps left in the budget when
    /// nothing shows each step (--trace, CONMA_PROFILE), and 0 otherwise. The
    /// steps are numbered as before; no other VProc runs between them, which
    /// changes only when VProcs are switched (FAST-PLAN 3.3).
    pub fast_steps: u64,
    /// Load `__Number_fromString__ onError "text" ,(x) ...` with the Number in place of x
    /// (step.rs, load_const; an experiment): off with CONMA_NO_FOLD=1.
    pub fold_constants: bool,
    /// The Rust calls of native code that are running (vm/src/native.rs).
    pub nctx: crate::transpiled::Ctx,
    /// (VProc, VProc) waiting for another VProc to finish.
    joining: Vec<(VProcRef, VProcRef)>,
    /// (VProc, LazyList) waiting for another VProc to evaluate a LazyList.
    lazy: Vec<(VProcRef, Rc<RefObj>)>,
    /// (VProc, Ref, value) waiting for the value of a Ref to change.
    changes: Vec<(VProcRef, Rc<RefObj>, Value)>,
    /// (VProc, OSProc or Stream) waiting for the OS.
    waiting: Vec<(VProcRef, OsTarget)>,
    /// Steps since the OS was last checked for the waiting VProcs.
    unpolled: u32,
    /// A process was added to waiting since the last check of the OS.
    os_dirty: bool,
    /// sys::signals() at the last check of the OS.
    signals_seen: u64,
    /// Read end of the pipe that SIGCHLD and a trapped SIGINT write to.
    wakeup: Option<i32>,
    /// The Ref of __OS_trap_interrupt__, and the SIGINTs counted so far.
    trap: Option<(Rc<RefObj>, u64)>,
    /// Streams of fds 0, 1, 2, made on first use.
    pub std_streams: [Option<Rc<StreamObj>>; 3],
    /// A VProc may have finished since joining was checked.
    pub finished_any: bool,
    /// The first VProc (serial 1) ended with an error: exit with code 1.
    pub main_failed: bool,
    /// A Ref may have changed since the VProcs waiting for a change were checked.
    pub ref_changed: bool,
    /// Serial numbers of the values that traces show by number.
    pub serials: Serials,
    next_serial: usize,
    true_closure: Value,
    false_closure: Value,
    select_closure: Value,
    nop_closure: Value,
    /// The LCont that step.rs (load_prim) gives a primitive it runs while loading a Body,
    /// to see whether the primitive returned to it: a Closure no program can get.
    pub inline_mark: Value,
    /// CONMA_SEED: choose the next VProc at random.
    rng: Option<SplitMix64>,
    /// Sleep in the OS while every VProc waits for it. False in the VM for
    /// wasm32, where run_slice returns Slice::Idle instead.
    pub blocking: bool,
}

/// How run_slice ended.
pub enum Slice {
    /// Every VProc has finished, or the exit code (as run returns it).
    Done(Option<i32>),
    /// The budget of steps was used up; some VProc can still run.
    Paused,
    /// No VProc can run until the outside changes something (input, a
    /// signal): the VM for wasm32 returns to its host.
    Idle,
}

/// The pseudo-random numbers of CONMA_SEED (the same as SplitMix64 in
/// test/vproc_step.py).
pub struct SplitMix64(u64);

impl SplitMix64 {
    /// A number in 0 .. n-1.
    fn below(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        ((z ^ (z >> 31)) % n as u64) as usize
    }
}

/// The SplitMix64 of CONMA_SEED, or None when it is not set.
fn seed_from_env() -> Option<SplitMix64> {
    let text = std::env::var("CONMA_SEED").unwrap_or_default();
    if text.is_empty() {
        return None;
    }
    match text.parse::<u64>() {
        Ok(n) if text.bytes().all(|b| b.is_ascii_digit()) => Some(SplitMix64(n)),
        _ => {
            write_std(2, &format!("Error: CONMA_SEED must be a number from 0 to {}: '{}'\n", u64::MAX, text));
            std::process::exit(2);
        }
    }
}

/// Steps between checks of the OS while some VProc is ready.
const POLL_INTERVAL: u32 = 64;

/// What a VProc waits for in the OS.
pub enum OsTarget {
    /// The process to exit; its exit code is given to the VProc.
    Proc(Rc<OSProcObj>),
    /// The Stream to be readable (input) or writable (output), or closed.
    Stream(Rc<StreamObj>),
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

/// The Closures that __if_then_else__, __if__ and __unless__ use: `,(clause)
/// clause`, which applies the clause a Boolean passes back, and `,()`, the
/// clause that does nothing. They have the AST that test/vproc_step.py builds.
fn select_closure() -> Value {
    let a = Sexp::atom;
    let l = Sexp::list;
    let p0 = Sexp::pos0;
    let body = l(vec![
        a("Body"),
        p0(),
        l(vec![a("Operator"), p0(), l(vec![a("Variable"), p0(), a("\"clause\""), l(vec![a("L"), a("0")])])]),
        l(vec![a("OList"), p0()]),
        l(vec![a("LCont"), p0()]),
    ]);
    Value::Closure(Rc::new(Closure {
        params: Rc::from(vec![Rc::from("clause")]),
        skip: 0,
        body: Some(compile_body(&body)),
        lvenv: None,
        gvenv: None,
        variadic: false,
    }))
}

fn nop_closure() -> Value {
    let a = Sexp::atom;
    let l = Sexp::list;
    let p0 = Sexp::pos0;
    let body = l(vec![
        a("Body"),
        p0(),
        l(vec![a("Operator"), p0(), l(vec![a("Null"), p0()])]),
        l(vec![a("OList"), p0()]),
        l(vec![a("LCont"), p0()]),
    ]);
    Value::Closure(Rc::new(Closure {
        params: Rc::from(Vec::<Rc<str>>::new()),
        skip: 0,
        body: Some(compile_body(&body)),
        lvenv: None,
        gvenv: None,
        variadic: false,
    }))
}

impl Machine {
    pub fn new(trace: bool) -> Machine {
        crate::gc::init();
        Machine {
            vprocs: Vec::new(),
            vprocs_pruned: 64,
            ready: VecDeque::new(),
            current: None,
            steps: 0,
            trace,
            transpiled_on: false,
            profile: crate::profile::Profile::from_env(),
            fast_steps: 0,
            fold_constants: std::env::var_os("CONMA_NO_FOLD").is_none(),
            nctx: Default::default(),
            joining: Vec::new(),
            lazy: Vec::new(),
            changes: Vec::new(),
            waiting: Vec::new(),
            unpolled: 0,
            os_dirty: false,
            signals_seen: 0,
            wakeup: None,
            trap: None,
            std_streams: [None, None, None],
            finished_any: false,
            main_failed: false,
            ref_changed: false,
            serials: Serials::default(),
            rng: None,
            blocking: true,
            next_serial: 0,
            true_closure: bool_closure("t"),
            false_closure: bool_closure("f"),
            select_closure: select_closure(),
            nop_closure: nop_closure(),
            inline_mark: nop_closure(),
        }
    }

    /// For a module of fast ConMa with the feature "boolean" (module::uses_boolean):
    /// the Booleans the primitives return are the primitives __TRUE__ and __FALSE__,
    /// not the Closures of concise ConMa.
    pub fn use_prim_booleans(&mut self) {
        self.true_closure = Value::Prim(crate::prim::lookup("__TRUE__").expect("__TRUE__"));
        self.false_closure = Value::Prim(crate::prim::lookup("__FALSE__").expect("__FALSE__"));
    }
    pub fn true_closure(&self) -> Value {
        self.true_closure.clone()
    }
    pub fn false_closure(&self) -> Value {
        self.false_closure.clone()
    }
    pub fn select_closure(&self) -> Value {
        self.select_closure.clone()
    }
    pub fn nop_closure(&self) -> Value {
        self.nop_closure.clone()
    }
    /// True if v is the Closure of nop_closure.
    pub fn is_nop(&self, v: &Value) -> bool {
        same(v, &self.nop_closure)
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
        crate::gc::new_vproc(VProcCell {
            serial,
            state: Cell::new(State::Ready),
            kill_pending: Cell::new(false),
            vp: RefCell::new(VProc::new(serial, operator, olist, source_file, gvenv, cwd)),
        })
    }

    pub fn add(&mut self, vp: VProcRef) {
        vp.state.set(State::Ready);
        // Forget the finished VProcs now and then: the list is only used to
        // report a deadlock, and it would keep every VProc alive.
        if self.vprocs.len() >= self.vprocs_pruned * 2 {
            self.vprocs.retain(|v| v.state.get() != State::Finished);
            self.vprocs_pruned = self.vprocs.len().max(64);
        }
        self.vprocs.push(vp.clone());
        self.ready.push_back(vp);
    }

    /// True if the next pass of the scheduler would only run cell again:
    /// no other VProc is ready, none waits, no SIGINT is trapped, no
    /// collection is due and cell is not to be killed.
    fn runs_alone(&self, cell: &VProcRef) -> bool {
        self.ready.is_empty()
            && self.joining.is_empty()
            && self.lazy.is_empty()
            && self.changes.is_empty()
            && self.waiting.is_empty()
            && self.trap.is_none()
            && !cell.kill_pending.get()
            && !crate::gc::due()
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

    /// Make SIGCHLD wake a wait for fds (once).
    pub fn install_sigchld(&mut self) {
        if self.wakeup.is_none() {
            self.wakeup = crate::sys::install_wakeup().ok();
        }
    }

    /// From now on SIGINT sets the value of r to the number of SIGINTs so far.
    pub fn trap_interrupt(&mut self, r: Rc<RefObj>) {
        self.install_sigchld();
        crate::sys::trap_sigint();
        let seen = crate::sys::interrupts();
        self.trap = Some((r, seen));
    }

    /// Set the Ref of __OS_trap_interrupt__ if a SIGINT came.
    fn check_interrupts(&mut self) {
        if let Some((r, seen)) = &mut self.trap {
            let n = crate::sys::interrupts();
            if n != *seen {
                *seen = n;
                *r.value.borrow_mut() = num(n as f64);
                self.ref_changed = true;
            }
        }
    }

    /// Make vp wait for the OS: until the process exits, or the Stream is
    /// readable (input) or writable (output).
    pub fn wait_os(&mut self, vp: &VProcRef, target: OsTarget) {
        if matches!(target, OsTarget::Proc(_)) {
            self.install_sigchld();
            self.os_dirty = true;
        }
        self.make_waiting(vp);
        self.waiting.push((vp.clone(), target));
    }

    /// Wake the VProcs whose process has exited or whose Stream is ready. If
    /// block, sleep until one of them can wake.
    fn check_waiting(&mut self, block: bool) {
        // While only processes wait, one can have exited only if a SIGCHLD came (or it was
        // added since the last check): skip the poll() and waitpid() calls otherwise.
        if !block && !self.os_dirty && self.waiting.iter().all(|(_, t)| matches!(t, OsTarget::Proc(_))) {
            if crate::sys::signals() == self.signals_seen {
                self.check_interrupts();
                return;
            }
        }
        self.os_dirty = false;
        self.signals_seen = crate::sys::signals();
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        let mut timeout = if block { None } else { Some(0) };
        for (_, t) in &self.waiting {
            if let OsTarget::Stream(s) = t {
                if s.closed.get() {
                    timeout = Some(0); // a closed Stream wakes its VProc at once
                } else if s.mode == 'r' {
                    reads.push(s.fd);
                } else {
                    writes.push(s.fd);
                }
            }
        }
        if let Some(w) = self.wakeup {
            reads.push(w);
        }
        let ready = if !reads.is_empty() || !writes.is_empty() || timeout == Some(0) {
            crate::sys::wait_fds(&reads, &writes, timeout).unwrap_or_default()
        } else {
            Vec::new()
        };
        if let Some(w) = self.wakeup {
            if ready.contains(&w) {
                crate::sys::drain(w);
            }
        }
        self.check_interrupts();
        let list = std::mem::take(&mut self.waiting);
        for (vp, t) in list {
            match &t {
                OsTarget::Proc(p) => match p.poll() {
                    Ok(Some(code)) => vp.vp.borrow_mut().olist.insert(0, num(code as f64)),
                    _ => {
                        self.waiting.push((vp, t));
                        continue;
                    }
                },
                OsTarget::Stream(s) => {
                    if !(s.closed.get() || ready.contains(&s.fd)) {
                        self.waiting.push((vp, t));
                        continue;
                    }
                }
            }
            self.wake(&vp);
        }
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
            if same(&r.value.borrow(), &value) {
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
        self.waiting.retain(|(v, _)| !Rc::ptr_eq(v, vp));
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
        self.seed();
        match self.run_slice(u64::MAX) {
            Slice::Done(code) => code,
            // Only without blocking, or after u64::MAX steps.
            Slice::Paused | Slice::Idle => None,
        }
    }

    /// Take CONMA_SEED (before the first step).
    pub fn seed(&mut self) {
        self.rng = seed_from_env();
    }

    /// Run at most about budget steps (the steps of the VProc running when
    /// the budget ends are not cut short in the middle of a step).
    pub fn run_slice(&mut self, budget: u64) -> Slice {
        let limit = self.steps.saturating_add(budget);
        loop {
            if self.steps >= limit {
                return Slice::Paused;
            }
            if crate::gc::due() {
                // Between two steps no RefCell is borrowed.
                self.vprocs.retain(|v| v.state.get() != State::Finished);
                crate::gc::collect();
            }
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
            self.check_interrupts();
            if !self.changes.is_empty() && self.ref_changed {
                self.ref_changed = false;
                self.check_changes();
            }
            if !self.waiting.is_empty() {
                if self.ready.is_empty() {
                    // Without a ready VProc, sleep in the OS instead of busy waiting.
                    self.unpolled = 0;
                    self.check_waiting(self.blocking);
                    if self.ready.is_empty() {
                        if !self.blocking {
                            return Slice::Idle;
                        }
                        continue;
                    }
                } else if self.unpolled >= POLL_INTERVAL {
                    self.unpolled = 0;
                    self.check_waiting(false);
                } else {
                    self.unpolled += 1;
                }
            }
            if self.ready.is_empty() && self.trap.is_some() && !self.changes.is_empty() {
                // A VProc may wait for the Ref of __OS_trap_interrupt__:
                // sleep until a signal arrives instead of reporting a deadlock.
                if !self.blocking {
                    return Slice::Idle;
                }
                self.check_waiting(true);
                continue;
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
                    return Slice::Done(None);
                }
                write_std(2, &format!("Deadlock: no VProc is ready; blocked VProc(s): {}\n", blocked.join(", ")));
                return Slice::Done(Some(2));
            }
            if let Some(rng) = &mut self.rng {
                if self.ready.len() > 1 {
                    // CONMA_SEED: move a VProc chosen at random to the front.
                    let i = rng.below(self.ready.len());
                    if i > 0 {
                        let chosen = self.ready.remove(i).unwrap();
                        self.ready.push_front(chosen);
                    }
                    if self.ready[0].vp.borrow().is_terminal() {
                        continue; // the top of the loop finishes it
                    }
                }
            }
            let cell = self.ready.pop_front().unwrap();
            cell.state.set(State::Running);
            self.current = Some(cell.clone());
            let mut vp = cell.vp.borrow_mut();
            let mut requeue = false;
            let may_fuse = !self.trace && self.profile.is_none();
            // While no other VProc can run and nothing waits, a pass of the
            // loop above would only take this VProc again: run its steps here.
            loop {
                if let Some(p) = &mut self.profile {
                    p.count(&vp);
                }
                self.fast_steps = if may_fuse { limit.saturating_sub(self.steps) } else { 0 };
                let r = step(self, &mut vp);
                match r {
                    Ok(()) => {
                        self.steps += 1;
                        if self.trace {
                            self.print_trace(&vp, self.steps);
                        }
                    }
                    Err(VmError::Error(e)) => {
                        write_std(2, &format!("Error in VProc {} at step {}: {}\n", vp.serial, self.steps, crate::step::format_error(&e, &vp.sinfo)));
                        if vp.serial == 1 {
                            self.main_failed = true;
                        }
                        self.steps += 1;
                        cell.state.set(State::Finished);
                        self.finished_any = true;
                    }
                    Err(VmError::Exit(code)) => {
                        self.current = None;
                        return Slice::Done(Some(code));
                    }
                }
                if cell.kill_pending.get() && cell.state.get() != State::Finished && !evaluating_lazy(&cell, &mut vp) {
                    self.kill_now(&cell);
                }
                if cell.state.get() != State::Running {
                    break;
                }
                if vp.is_terminal() {
                    cell.state.set(State::Finished);
                    self.finished_any = true;
                    break;
                }
                if !self.runs_alone(&cell) || self.steps >= limit {
                    requeue = true;
                    break;
                }
            }
            self.current = None;
            if requeue {
                cell.state.set(State::Ready);
                drop(vp);
                self.ready.push_back(cell);
            }
        }
    }
}

