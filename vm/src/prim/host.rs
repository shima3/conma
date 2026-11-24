//! __Host_call__ onError name args ,(result): call the function of the host
//! named name (docs/dev/PLATFORM-PLAN-ja.md 3.2, stage P2). The values that go
//! to the host and come back are Numbers, Strings, Booleans, Dicts with String
//! keys and LLists of them (null is the empty LList; Booleans and Dicts since
//! stage P7, 4.9 of the plan). The functions are those of the table Host that
//! the Machine has: Host::new gives those every VM has (the feature "test", and
//! "time" on the native and transpiled VMs; vm/python/vproc_step.py has the
//! same), and a program that embeds the VM adds its own (Host::function,
//! Host::waiting, Host::feature; src/lib.rs, docs/HOST-ja.md). The VM for wasm32
//! also asks its host for the other names (sys::host_call, the hook
//! hostFunctions of vm/wasm/conma.js).
//!
//! __Host_start__, __Host_start_tracked__ and __Host_result__ are the calls
//! that wait (3.4 of the plan, stage P3): __Host_start__ returns the read end
//! of a pipe, which the program waits for with __OS_wait_readable__; the pipe
//! is at its end when the call has finished. The result of a function of Host
//! is kept in the Machine (HostResults, by the serial of the Stream), and the
//! pipe is only closed, so that a result of any size can be given (4.9 of the
//! plan; before, it was written to the pipe, at most 8192 bytes). A function of
//! Host::function finishes at once; a function of Host::waiting finishes when
//! its Done is finished, maybe from another thread: the scheduler finishes the
//! call between two steps (HostResults::take_finished). test.later finishes by
//! the virtual clock of Later, the same on every VM, and time.sleep by the real
//! clock (Timers). On the VM for wasm32 the other names go to the host
//! (sys::host_start), which writes the JSON text of the result ({"r":value} or
//! {"e":"message"}) to the pipe when the function (maybe a Promise) has finished.

use super::stream::{check_stream, new_stream};
use super::{take_n, PResult, VmError};
use crate::sched::Machine;
use crate::sys;
use crate::value::*;
use crate::vproc::VProc;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// A value that goes to the host or comes back.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Num(f64),
    Str(String),
    /// An LList; the empty one is null.
    List(Vec<HostValue>),
    Bool(bool),
    /// A Dict, its String keys in the order they were put in.
    Dict(Vec<(String, HostValue)>),
}

type CallFn = Box<dyn Fn(&[HostValue]) -> Result<HostValue, String> + Send>;
type WaitFn = Box<dyn Fn(&[HostValue], Done) + Send>;

enum HostFn {
    /// returns at once (Host::function)
    Call(CallFn),
    /// finishes later, with its Done (Host::waiting)
    Wait(WaitFn),
}

/// The functions of the host that a VM can call, and the platform features they give
/// (docs/HOST-ja.md).
pub struct Host {
    fns: HashMap<String, HostFn>,
    features: Vec<String>,
    shared: Arc<Shared>,
}

/// What a Host shares with the Done of its calls, which may be finished from other threads, also
/// after the VM has ended: the results finished and not yet taken by the scheduler, and a pipe
/// whose read end the scheduler polls, to which a finish writes a byte. The pipe is closed when
/// the Host (in the Machine) and the last Done have gone, so that a late finish never writes to an
/// fd that has been reused.
struct Shared {
    finished: Mutex<Vec<(usize, Result<HostValue, String>)>>,
    any: AtomicBool,
    wake: Mutex<Option<(i32, i32)>>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        if let Some((r, w)) = self.wake.get_mut().ok().and_then(|o| o.take()) {
            let _ = sys::close_fd(r);
            let _ = sys::close_fd(w);
        }
    }
}

/// The end of a call of a function of Host::waiting: call finish once, from any thread, when the
/// function has finished. A Done dropped without finish finishes the call with an error.
pub struct Done {
    shared: Arc<Shared>,
    id: usize,
    name: String,
    finished: bool,
}

impl Done {
    /// Finish the call with result: a value, or the message of an error.
    pub fn finish(mut self, result: Result<HostValue, String>) {
        self.finished = true;
        self.shared.push(self.id, result);
    }
}

impl Drop for Done {
    fn drop(&mut self) {
        if !self.finished {
            self.shared.push(self.id, Err(format!("{}: the host dropped the call", self.name)));
        }
    }
}

impl Shared {
    fn push(&self, id: usize, result: Result<HostValue, String>) {
        if let Ok(mut f) = self.finished.lock() {
            f.push((id, result));
        }
        self.any.store(true, Ordering::SeqCst);
        if let Ok(w) = self.wake.lock() {
            if let Some((_, w)) = *w {
                // the write end does not block: a full pipe wakes the scheduler anyway
                let _ = sys::write_fd(w, b"x");
            }
        }
    }
}

impl Default for Host {
    fn default() -> Self {
        Host::new()
    }
}

impl Host {
    /// The functions every VM has: those of the feature "test", for the tests (test/fast14,
    /// test/fast15; test.later, which waits by the virtual clock, is handled by __Host_start__),
    /// and on the native and transpiled VMs time.now of the feature "time" (docs/dev/PLATFORM-PLAN-ja.md
    /// 4.4, stage P5; time.sleep, which waits by the real clock, is handled by __Host_start__; the
    /// VM for wasm32, which has no clock, asks its host, vm/wasm/conma.js).
    pub fn new() -> Host {
        let mut h = Host {
            fns: HashMap::new(),
            features: vec!["test".to_string()],
            shared: Arc::new(Shared { finished: Mutex::new(Vec::new()), any: AtomicBool::new(false), wake: Mutex::new(None) }),
        };
        h.function("test.add", test_add);
        h.function("test.echo", |args: &[HostValue]| Ok(HostValue::List(args.to_vec())));
        h.function("test.fail", |_: &[HostValue]| Err("test.fail: failed".to_string()));
        #[cfg(not(target_arch = "wasm32"))]
        {
            h.function("time.now", time_now);
            h.feature("time");
        }
        h
    }

    /// Add (or replace) the function named name, which returns at once: __Host_call__ and
    /// __Host_start__ call it with the arguments, and pass back its value, or go to onError with
    /// its message. It runs in the thread of the VM, and the VM waits for it.
    pub fn function(&mut self, name: &str, f: impl Fn(&[HostValue]) -> Result<HostValue, String> + Send + 'static) -> &mut Host {
        self.fns.insert(name.to_string(), HostFn::Call(Box::new(f)));
        self
    }

    /// Add (or replace) the function named name, which waits: __Host_start__ calls it with the
    /// arguments and a Done, and the call finishes when the Done is finished (from any thread);
    /// the other VProcs run in the meantime. f itself should return soon (it runs in the
    /// thread of the VM): it starts the work, for example in a thread that gets the Done.
    /// __Host_call__ cannot call it.
    pub fn waiting(&mut self, name: &str, f: impl Fn(&[HostValue], Done) + Send + 'static) -> &mut Host {
        self.fns.insert(name.to_string(), HostFn::Wait(Box::new(f)));
        self
    }

    /// Make __Feature_available__ true for the platform feature name.
    pub fn feature(&mut self, name: &str) -> &mut Host {
        if !self.features.iter().any(|f| f == name) {
            self.features.push(name.to_string());
        }
        self
    }

    /// True if name is a platform feature of these functions, or of the host of the VM for wasm32.
    pub fn has_feature(&self, name: &str) -> bool {
        self.features.iter().any(|f| f == name) || sys::host_feature(name)
    }

    fn has(&self, name: &str) -> bool {
        self.fns.contains_key(name)
    }

    /// The read end of the pipe that a finish writes to, once a function of Host::waiting has
    /// been called.
    pub fn wake_fd(&self) -> Option<i32> {
        self.shared.wake.lock().ok().and_then(|w| w.map(|(r, _)| r))
    }

    /// True if some call has been finished since the last take_finished.
    #[inline]
    pub fn any_finished(&self) -> bool {
        self.shared.any.load(Ordering::Relaxed)
    }

    fn done(&self, id: usize, name: &str) -> Result<Done, String> {
        let mut wake = self.shared.wake.lock().map_err(|_| "the host is broken".to_string())?;
        if wake.is_none() {
            *wake = Some(sys::make_wake_pipe().map_err(|e| sys::errno_str(&e))?);
        }
        Ok(Done { shared: self.shared.clone(), id, name: name.to_string(), finished: false })
    }
}

thread_local! {
    /// The Host that run_main_with gives to the VM of this thread (src/lib.rs).
    static NEXT_HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
}

/// Give host to the next Machine made in this thread.
pub fn set_host(host: Host) {
    NEXT_HOST.with(|h| *h.borrow_mut() = Some(host));
}

/// The Host for a new Machine: the one of set_host, or Host::new().
pub fn take_host() -> Host {
    NEXT_HOST.with(|h| h.borrow_mut().take()).unwrap_or_default()
}

/// test.add: the sum of its arguments, Numbers, from the left (0 for none).
fn test_add(args: &[HostValue]) -> Result<HostValue, String> {
    let mut sum = 0.0;
    for a in args {
        match a {
            HostValue::Num(x) => sum += x,
            _ => return Err("test.add: expected Numbers".into()),
        }
    }
    Ok(HostValue::Num(sum))
}

/// time.now: the time in milliseconds since 1970-01-01 00:00 UTC.
#[cfg(not(target_arch = "wasm32"))]
fn time_now(_args: &[HostValue]) -> Result<HostValue, String> {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|_| "time.now: the clock is before 1970")?;
    Ok(HostValue::Num(d.as_secs_f64() * 1000.0))
}

/// time.sleep ms waits on the native and transpiled VMs (on the VM for wasm32 the host does it).
const SLEEP: &str = "time.sleep";

/// The calls of __Host_start__ that have not been finished, and the results of those that have
/// and that __Host_result__ has not taken yet, by the serial of the Stream of the call.
#[derive(Default)]
pub struct HostResults {
    /// a call of Host::waiting, Timers or Later: the write end of its pipe, and its Stream
    calls: HashMap<usize, (i32, Weak<StreamObj>)>,
    results: HashMap<usize, (Weak<StreamObj>, Result<HostValue, String>)>,
}

impl HostResults {
    fn wait(&mut self, id: usize, w: i32, s: &Rc<StreamObj>) {
        self.calls.insert(id, (w, Rc::downgrade(s)));
    }

    /// Finish the call id with result: keep the result, and close the pipe (which wakes the
    /// reader). A call whose Stream has been closed (its VProc was killed) drops the result.
    fn finish(&mut self, id: usize, result: Result<HostValue, String>) {
        let Some((w, s)) = self.calls.remove(&id) else { return };
        let _ = sys::close_fd(w);
        if s.upgrade().is_some_and(|s| !s.closed.get()) {
            // forget the results and the calls of Streams closed meanwhile
            self.results.retain(|_, (s, _)| s.upgrade().is_some_and(|s| !s.closed.get()));
            self.calls.retain(|_, (w, s)| {
                let open = s.upgrade().is_some_and(|s| !s.closed.get());
                if !open {
                    let _ = sys::close_fd(*w);
                }
                open
            });
            self.results.insert(id, (s, result));
        }
    }

    /// Finish the calls of Host::waiting whose Done has been finished.
    pub fn take_finished(&mut self, host: &Host) {
        host.shared.any.store(false, Ordering::SeqCst);
        let list = match host.shared.finished.lock() {
            Ok(mut f) => std::mem::take(&mut *f),
            Err(_) => return,
        };
        for (id, r) in list {
            self.finish(id, r);
        }
    }
}

/// The calls of time.sleep that have not finished, by the real clock. The scheduler finishes those
/// whose time has come between two steps (sched.rs), and sleeps in poll() no longer than until
/// the next one.
#[derive(Default)]
pub struct Timers {
    /// (when, serial of the Stream of the call)
    jobs: Vec<(std::time::Instant, usize)>,
}

impl Timers {
    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn add(&mut self, ms: f64, id: usize) {
        let d = std::time::Duration::from_secs_f64(ms.min(1e12) / 1000.0);
        self.jobs.push((std::time::Instant::now() + d, id));
    }

    /// Finish the calls whose time has come.
    pub fn finish_due(&mut self, results: &mut HostResults) {
        let now = std::time::Instant::now();
        let mut i = 0;
        while i < self.jobs.len() {
            if self.jobs[i].0 <= now {
                let (_, id) = self.jobs.remove(i);
                results.finish(id, Ok(HostValue::List(Vec::new())));
            } else {
                i += 1;
            }
        }
    }

    /// The milliseconds until the next call is due (rounded up), or None if there is none.
    pub fn timeout_ms(&self) -> Option<i32> {
        let now = std::time::Instant::now();
        self.jobs
            .iter()
            .map(|(t, _)| t.saturating_duration_since(now))
            .min()
            .map(|d| d.as_micros().div_ceil(1000).min(i32::MAX as u128) as i32)
    }
}

/// The value v as a value for the host; Err with the type name of the first value that cannot
/// go (a Closure that is not a Boolean, a Dict with a key that is not a String, a Dict that contains
/// itself, ...).
fn to_host(v: &Value) -> Result<HostValue, String> {
    to_host_in(v, &mut Vec::new())
}

/// to_host, with the Dicts being converted around v (a Dict among them again would never end).
fn to_host_in(v: &Value, path: &mut Vec<*const DictObj>) -> Result<HostValue, String> {
    if v.is_null() {
        return Ok(HostValue::List(Vec::new()));
    }
    if let Some(b) = super::control::boolean_choice(v) {
        return Ok(HostValue::Bool(b));
    }
    match v {
        Value::Number(x) => Ok(HostValue::Num(*x)),
        Value::LList(_) => {
            let mut items = Vec::new();
            let mut cur = v.clone();
            loop {
                if cur.is_null() {
                    break;
                }
                let next = match &cur {
                    Value::LList(c) => {
                        items.push(to_host_in(&c.head, path)?);
                        c.tail.clone()
                    }
                    other => return Err(value_type_name(other)),
                };
                cur = next;
            }
            Ok(HostValue::List(items))
        }
        Value::Dict(d) => {
            let p = Rc::as_ptr(d);
            if path.contains(&p) {
                return Err("a Dict that contains itself".to_string());
            }
            path.push(p);
            let mut entries = Vec::new();
            let list = d.entries.borrow().clone();
            for (k, x) in list.iter() {
                let key = match string_cow(k) {
                    Some(Ok(s)) => s.into_owned(),
                    Some(Err(e)) => return Err(e),
                    None => return Err(format!("Dict with a {} key", value_type_name(k))),
                };
                entries.push((key, to_host_in(x, path)?));
            }
            path.pop();
            Ok(HostValue::Dict(entries))
        }
        _ => match string_cow(v) {
            Some(Ok(s)) => Ok(HostValue::Str(s.into_owned())),
            Some(Err(e)) => Err(e),
            None => Err(value_type_name(v)),
        },
    }
}

/// A value of the host as a value of ConMa.
fn from_host(m: &mut Machine, h: &HostValue) -> Result<Value, VmError> {
    Ok(match h {
        HostValue::Num(x) => num(*x),
        HostValue::Str(s) => string_value(s),
        HostValue::Bool(b) => {
            if *b {
                m.true_closure()
            } else {
                m.false_closure()
            }
        }
        HostValue::List(items) => {
            let mut tail = Value::Null;
            for head in items.iter().rev() {
                tail = Value::LList(Rc::new(LList { head: from_host(m, head)?, tail }));
            }
            tail
        }
        HostValue::Dict(entries) => {
            m.serials.dicts += 1;
            let d = Value::Dict(crate::gc::new_dict(DictObj::new(m.serials.dicts)));
            for (k, x) in entries {
                let x = from_host(m, x)?;
                super::collections::dict_put_core(&d, string_value(k), x)?;
            }
            d
        }
    })
}

/// The name and the arguments of a call of the host (the arguments a, after
/// onError, of __Host_call__ and __Host_start__); None after going to onError.
fn name_args(vp: &mut VProc, prim: &str, on_error: &Value, name: &Value, args: &Value) -> Option<(String, Vec<HostValue>)> {
    let Some(Ok(fname)) = string_cow(name) else {
        vp.prim_error(on_error.clone(), &format!("{}: expected String name, got {}", prim, value_type_name(name)));
        return None;
    };
    let expected = "expected an LList of Numbers, Strings, Booleans, Dicts and LLists";
    match to_host(args) {
        Ok(HostValue::List(items)) => Some((fname.into_owned(), items)),
        Ok(_) => {
            vp.prim_error(on_error.clone(), &format!("{}: {}, got {}", prim, expected, value_type_name(args)));
            None
        }
        Err(t) => {
            vp.prim_error(on_error.clone(), &format!("{}: {}, got {}", prim, expected, t));
            None
        }
    }
}

/// What a call back from the host into the VM needs of the VProc that called the host (stage H1 of
/// docs/dev/CONJS-HOST-PLAN.md): its global variables, its PDict (the state of the runtime of ConJS,
/// which the call back reads and may change), its current directory, and the Booleans and the
/// transpiled code of its Machine. __Host_call__ pushes it while the function of the host runs.
// Only the VM for wasm32 calls back (vm/wasm/src/lib.rs, conma_callback): the native VMs have no host
// that runs during __Host_call__ and could call back.
#[allow(dead_code)]
struct CallbackCtx {
    gvenv: Rc<crate::module::GVEnv>,
    pdict: Vec<(Value, Value)>,
    cwd: String,
    booleans: (Value, Value),
    transpiled_on: bool,
}

thread_local! {
    static CALLBACKS: std::cell::RefCell<Vec<CallbackCtx>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// How a call back failed.
#[allow(dead_code)]
pub enum CallbackError {
    /// The function went to an error, or the call back was made when no function of the host runs.
    Error(String),
    /// The function came to something that waits (or started a VProc): the host cannot wait in the
    /// middle of its JavaScript (3.1 of the plan), so the call back stops there.
    Escape(String),
}

/// Run f, a call of the host (sys), so that the host can call the VM back (callback): it gets what it
/// needs of vp, and vp gets back its PDict as the call back left it.
pub fn with_callback_context<T>(m: &Machine, vp: &mut VProc, f: impl FnOnce() -> T) -> T {
    CALLBACKS.with(|c| {
        c.borrow_mut().push(CallbackCtx {
            gvenv: vp.gvenv.clone(),
            pdict: std::mem::take(&mut vp.pdict),
            cwd: vp.cwd.clone(),
            booleans: (m.true_closure(), m.false_closure()),
            transpiled_on: m.transpiled_on,
        })
    });
    let r = f();
    if let Some(ctx) = CALLBACKS.with(|c| c.borrow_mut().pop()) {
        vp.pdict = ctx.pdict;
    }
    r
}

/// The global variable gid of the VProc whose call of the host is running, or None.
#[allow(dead_code)]
pub fn callback_global(gid: usize) -> Option<Value> {
    CALLBACKS.with(|c| c.borrow().last().and_then(|ctx| ctx.gvenv.get(gid)))
}

/// Run f applied to args to its end, in a Machine of its own, while the VM is inside a call of the
/// host (a nested run; stage H1 of docs/dev/CONJS-HOST-PLAN.md). The VProc of the call back gets the
/// PDict of the VProc that called the host, which gets it back with what the call back changed.
/// Nothing else runs meanwhile: the scheduler of the VM is set aside, and the collector does not run.
#[allow(dead_code)]
pub fn callback(f: Value, args: &[HostValue]) -> Result<HostValue, CallbackError> {
    let Some((gvenv, pdict, cwd, booleans, transpiled_on)) = CALLBACKS.with(|c| {
        c.borrow().last().map(|x| (x.gvenv.clone(), x.pdict.clone(), x.cwd.clone(), x.booleans.clone(), x.transpiled_on))
    }) else {
        return Err(CallbackError::Error("a call back of the VM while no function of the host runs".to_string()));
    };
    let mut m = Machine::new(false);
    m.nested = true;
    m.blocking = false;
    m.transpiled_on = transpiled_on;
    m.set_booleans(booleans.0, booleans.1);
    let mut olist = Vec::with_capacity(args.len());
    for a in args {
        match from_host(&mut m, a) {
            Ok(v) => olist.push(v),
            Err(_) => return Err(CallbackError::Error("a call back of the VM: an argument cannot be made".to_string())),
        }
    }
    let vp = m.make_vproc(f, olist, None, gvenv, cwd);
    vp.vp.borrow_mut().pdict = pdict;
    m.add(vp.clone());
    crate::gc::nested_enter();
    let outcome = loop {
        match m.run_slice(1 << 20) {
            crate::sched::Slice::Paused => continue,
            crate::sched::Slice::Idle => break Err(CallbackError::Escape("the call back waits".to_string())),
            crate::sched::Slice::Done(Some(code)) => break Err(CallbackError::Error(format!("the call back exited with status {}", code))),
            crate::sched::Slice::Done(None) => break Ok(()),
        }
    };
    crate::gc::nested_leave();
    outcome?;
    if m.main_failed {
        return Err(CallbackError::Error("the call back stopped with an error".to_string()));
    }
    if m.vprocs.iter().any(|v| !Rc::ptr_eq(v, &vp) && v.state.get() != crate::vproc::State::Finished) {
        return Err(CallbackError::Escape("the call back started a VProc".to_string()));
    }
    let inner = vp.vp.borrow();
    CALLBACKS.with(|c| {
        if let Some(ctx) = c.borrow_mut().last_mut() {
            ctx.pdict = inner.pdict.clone();
        }
    });
    let result = match inner.olist.as_slice() {
        [v] => to_host(v),
        vs => vs.iter().map(to_host).collect::<Result<Vec<_>, _>>().map(HostValue::List),
    };
    result.map_err(|t| CallbackError::Error(format!("the call back returned a value that cannot go to the host: {}", t)))
}

/// __Host_call__ onError name args ,(result)
pub fn call(m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__Host_call__";
    let mut a = take_n(vp, prim, 3)?;
    let args = a.pop().unwrap();
    let name = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Some((fname, args)) = name_args(vp, prim, &on_error, &name, &args) else { return Ok(()) };
    let result = match m.host.fns.get(&fname) {
        Some(HostFn::Call(f)) => f(&args),
        Some(HostFn::Wait(_)) => Err(format!("{}: {} waits: call it with __Host_start__", prim, fname)),
        // The function of the host may call the VM back (callback)
        None => match with_callback_context(m, vp, || sys::host_call(&fname, &to_json(&HostValue::List(args)))) {
            None => Err(format!("{}: no host function \"{}\"", prim, fname)),
            Some(Err(message)) => Err(message),
            Some(Ok(json)) => match from_json(&json) {
                Ok(r) => r,
                Err(e) => Err(format!("{}: {} returned {}", prim, fname, e)),
            },
        },
    };
    match result {
        Ok(h) => {
            let v = from_host(m, &h)?;
            vp.prim_return1(v)
        }
        Err(message) => vp.prim_error(on_error, &message),
    }
    Ok(())
}

/// The calls of test.later that have not finished, by the virtual clock: when
/// no VProc can run, the call with the earliest deadline (then the first
/// called) finishes, and the clock moves to its deadline (3.4 of the plan).
#[derive(Default)]
pub struct Later {
    clock: f64,
    calls: u64,
    /// (deadline, number of the call, serial of the Stream of the call, result)
    jobs: Vec<(f64, u64, usize, HostValue)>,
}

impl Later {
    /// Finish the next call of test.later; false if there is none.
    pub fn finish_next(&mut self, results: &mut HostResults) -> bool {
        let Some(i) = (0..self.jobs.len()).min_by(|&a, &b| {
            let (x, y) = (&self.jobs[a], &self.jobs[b]);
            x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal).then(x.1.cmp(&y.1))
        }) else {
            return false;
        };
        let (deadline, _, id, value) = self.jobs.remove(i);
        self.clock = deadline;
        results.finish(id, Ok(value));
        true
    }
}

/// __Host_start__ onError name args ,(inputStream)
pub fn start(m: &mut Machine, vp: &mut VProc) -> PResult {
    start_with(m, vp, "__Host_start__", false)
}

/// __Host_start_tracked__ onError name args Ref ,(inputStream): __Host_start__,
/// and in the same step, put the Stream in front of the LList in the Ref (as
/// __OS_open_tracked__), so that a killed VProc leaves no pipe open.
pub fn start_tracked(m: &mut Machine, vp: &mut VProc) -> PResult {
    start_with(m, vp, "__Host_start_tracked__", true)
}

fn start_with(m: &mut Machine, vp: &mut VProc, prim: &str, tracked: bool) -> PResult {
    let mut a = take_n(vp, prim, if tracked { 4 } else { 3 })?;
    let track = if tracked { a.pop() } else { None };
    let args = a.pop().unwrap();
    let name = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let track = match track {
        None => None,
        Some(Value::Ref(r)) if matches!(&*r.value.borrow(), Value::Null | Value::LList(_)) => Some(r),
        Some(v) => {
            let got = match &v {
                Value::Ref(r) => format!("a Ref of {}", value_type_name(&r.value.borrow())),
                other => value_type_name(other).to_string(),
            };
            vp.prim_error(on_error, &format!("{}: expected a Ref of an LList, got {}", prim, got));
            return Ok(());
        }
    };
    let Some((fname, args)) = name_args(vp, prim, &on_error, &name, &args) else { return Ok(()) };
    let later = fname == "test.later";
    let sleep = cfg!(not(target_arch = "wasm32")) && fname == SLEEP;
    if sleep {
        match args.first() {
            Some(HostValue::Num(n)) if *n >= 0.0 => {}
            _ => {
                vp.prim_error(on_error, &format!("{}: time.sleep: expected milliseconds (a Number, 0 or more)", prim));
                return Ok(());
            }
        }
    } else if later {
        match args.first() {
            Some(HostValue::Num(n)) if *n >= 0.0 => {}
            _ => {
                vp.prim_error(on_error, &format!("{}: test.later: expected a count (a Number, 0 or more) first", prim));
                return Ok(());
            }
        }
    } else if !m.host.has(&fname) && !sys::host_has(&fname) {
        vp.prim_error(on_error, &format!("{}: no host function \"{}\"", prim, fname));
        return Ok(());
    }
    let (r, w) = sys::make_pipe().map_err(|e| VmError::Error(format!("{}: {}", prim, sys::errno_str(&e))))?;
    let stream = new_stream(m, r, 'r');
    let id = stream.serial;
    if later {
        let deadline = m.host_later.clock + if let Some(HostValue::Num(n)) = args.first() { *n } else { 0.0 };
        m.host_later.calls += 1;
        let n = m.host_later.calls;
        m.host_results.wait(id, w, &stream);
        m.host_later.jobs.push((deadline, n, id, HostValue::List(args)));
    } else if sleep {
        // the scheduler finishes it when the time has come (Timers)
        let ms = if let Some(HostValue::Num(n)) = args.first() { *n } else { 0.0 };
        m.host_results.wait(id, w, &stream);
        m.host_timers.add(ms, id);
    } else {
        match m.host.fns.get(&fname) {
            Some(HostFn::Call(f)) => {
                let r = f(&args);
                m.host_results.wait(id, w, &stream);
                m.host_results.finish(id, r);
            }
            Some(HostFn::Wait(f)) => match m.host.done(id, &fname) {
                Ok(done) => {
                    m.host_results.wait(id, w, &stream);
                    f(&args, done);
                }
                Err(e) => {
                    let _ = sys::close_fd(r);
                    let _ = sys::close_fd(w);
                    stream.closed.set(true);
                    vp.prim_error(on_error, &format!("{}: {}", prim, e));
                    return Ok(());
                }
            },
            None => sys::host_start(&fname, &to_json(&HostValue::List(args)), w),
        }
    }
    let s = Value::Stream(stream);
    if let Some(t) = track {
        let tail = std::mem::replace(&mut *t.value.borrow_mut(), Value::Null);
        *t.value.borrow_mut() = Value::LList(Rc::new(LList { head: s.clone(), tail }));
        m.ref_changed = true;
    }
    vp.prim_return1(s);
    Ok(())
}

/// __Host_result__ onError inputStream ,(result): the result of the call of
/// __Host_start__ that gave inputStream, which is then closed. onError if the
/// call failed, or has not finished (the pipe is not at its end).
pub fn result(m: &mut Machine, vp: &mut VProc) -> PResult {
    let prim = "__Host_result__";
    let mut a = take_n(vp, prim, 2)?;
    let v = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let Some(s) = check_stream(vp, &on_error, prim, &v, Some('r')) else { return Ok(()) };
    let mut text = Vec::new();
    let mut buf = [0u8; 4096];
    let ended = loop {
        match sys::readable(s.fd).and_then(|ok| if ok { sys::read_fd(s.fd, &mut buf).map(Some) } else { Ok(None) }) {
            Ok(Some(0)) => break Ok(true),
            Ok(Some(n)) => text.extend_from_slice(&buf[..n]),
            Ok(None) => break Ok(false),
            Err(e) => break Err(e),
        }
    };
    let r = match ended {
        Err(e) => Err(format!("{}: {}", prim, sys::errno_str(&e))),
        Ok(false) => Err(format!("{}: the call has not finished", prim)),
        Ok(true) => {
            let _ = s.close();
            match m.host_results.results.remove(&s.serial) {
                // a function of Host, test.later or time.sleep
                Some((_, r)) => r,
                // the host of the VM for wasm32 wrote the result
                None => match from_json(&String::from_utf8_lossy(&text)) {
                    Ok(r) => r,
                    Err(e) => Err(format!("{}: the host gave {}", prim, e)),
                },
            }
        }
    };
    match r {
        Ok(h) => {
            let v = from_host(m, &h)?;
            vp.prim_return1(v)
        }
        Err(message) => vp.prim_error(on_error, &message),
    }
    Ok(())
}

// The values go to the host of the VM for wasm32 and come back as JSON: a
// Number that JSON cannot write (-0, NaN, an infinity) as {"n":"-0"},
// {"n":"NaN"}, {"n":"Infinity"} or {"n":"-Infinity"}, null for the empty LList,
// true and false for the Booleans, and a Dict as {"d":{"key":value,...}}.
// A result is {"r":value} or {"e":"message"} (vm/wasm/conma.js, hostCall).

/// The JSON of a value for the host.
pub fn to_json(h: &HostValue) -> String {
    let mut out = String::new();
    write_json(h, &mut out);
    out
}

fn write_json(h: &HostValue, out: &mut String) {
    match h {
        HostValue::Num(x) => {
            if x.is_nan() {
                out.push_str("{\"n\":\"NaN\"}");
            } else if x.is_infinite() {
                out.push_str(if *x > 0.0 { "{\"n\":\"Infinity\"}" } else { "{\"n\":\"-Infinity\"}" });
            } else if *x == 0.0 && x.is_sign_negative() {
                out.push_str("{\"n\":\"-0\"}");
            } else {
                // Rust writes the shortest digits that read back as the same f64, as JSON needs
                out.push_str(&format!("{:?}", x));
            }
        }
        HostValue::Str(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        HostValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        HostValue::Dict(entries) => {
            // wrapped, so that a Dict is not taken for a Number such as {"n":"NaN"}
            out.push_str("{\"d\":{");
            for (i, (k, x)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(&HostValue::Str(k.clone()), out);
                out.push(':');
                write_json(x, out);
            }
            out.push_str("}}");
        }
        HostValue::List(items) if items.is_empty() => out.push_str("null"),
        HostValue::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
    }
}

/// A result of the host: Ok(Ok(value)) for {"r":value}, Ok(Err(message)) for
/// {"e":"message"}, and Err for a text that is neither.
pub fn from_json(text: &str) -> Result<Result<HostValue, String>, String> {
    let mut p = Json { s: text.as_bytes(), i: 0 };
    p.ws();
    p.expect(b'{')?;
    p.ws();
    let key = p.string()?;
    p.ws();
    p.expect(b':')?;
    p.ws();
    let r = match key.as_str() {
        "r" => Ok(p.value()?),
        "e" => Err(p.string()?),
        _ => return Err(format!("an object with the key \"{}\"", key)),
    };
    p.ws();
    p.expect(b'}')?;
    p.ws();
    if p.i != p.s.len() {
        return Err("more than one value".into());
    }
    Ok(r)
}

const VALUE_ERROR: &str = "a value that is not a Number, a String, a Boolean, a Dict or a list";

struct Json<'a> {
    s: &'a [u8],
    i: usize,
}

impl Json<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("a JSON text that is not a result (at byte {})", self.i))
        }
    }

    fn value(&mut self) -> Result<HostValue, String> {
        self.ws();
        match self.s.get(self.i) {
            Some(b'"') => Ok(HostValue::Str(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.s.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(HostValue::List(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.s.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(HostValue::List(items));
                        }
                        _ => return Err(format!("a JSON text that is not a result (at byte {})", self.i)),
                    }
                }
            }
            Some(b'n') if self.s[self.i..].starts_with(b"null") => {
                self.i += 4;
                Ok(HostValue::List(Vec::new()))
            }
            Some(b't') if self.s[self.i..].starts_with(b"true") => {
                self.i += 4;
                Ok(HostValue::Bool(true))
            }
            Some(b'f') if self.s[self.i..].starts_with(b"false") => {
                self.i += 5;
                Ok(HostValue::Bool(false))
            }
            Some(b'{') => {
                self.i += 1;
                self.ws();
                let key = self.string()?;
                self.ws();
                self.expect(b':')?;
                self.ws();
                if key == "d" {
                    // a Dict: {"d":{"key":value,...}}
                    self.expect(b'{')?;
                    let mut entries: Vec<(String, HostValue)> = Vec::new();
                    self.ws();
                    if self.s.get(self.i) == Some(&b'}') {
                        self.i += 1;
                    } else {
                        loop {
                            self.ws();
                            let k = self.string()?;
                            self.ws();
                            self.expect(b':')?;
                            let x = self.value()?;
                            match entries.iter_mut().find(|(e, _)| *e == k) {
                                Some(e) => e.1 = x,
                                None => entries.push((k, x)),
                            }
                            self.ws();
                            match self.s.get(self.i) {
                                Some(b',') => self.i += 1,
                                Some(b'}') => {
                                    self.i += 1;
                                    break;
                                }
                                _ => return Err(format!("a JSON text that is not a result (at byte {})", self.i)),
                            }
                        }
                    }
                    self.ws();
                    self.expect(b'}')?;
                    return Ok(HostValue::Dict(entries));
                }
                let v = self.string()?;
                self.ws();
                self.expect(b'}')?;
                let x = match (key.as_str(), v.as_str()) {
                    ("n", "-0") => -0.0,
                    ("n", "NaN") => f64::NAN,
                    ("n", "Infinity") => f64::INFINITY,
                    ("n", "-Infinity") => f64::NEG_INFINITY,
                    _ => return Err(VALUE_ERROR.into()),
                };
                Ok(HostValue::Num(x))
            }
            Some(c) if *c == b'-' || c.is_ascii_digit() => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[start..self.i]).unwrap();
                t.parse::<f64>().map(HostValue::Num).map_err(|_| format!("the number {}", t))
            }
            _ => Err(VALUE_ERROR.into()),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let Some(&c) = self.s.get(self.i) else { return Err("a JSON string without its end".into()) };
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let Some(&e) = self.s.get(self.i) else { return Err("a JSON string without its end".into()) };
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut code = self.hex4()?;
                            if (0xD800..0xDC00).contains(&code) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let low = self.hex4()?;
                                code = 0x10000 + ((code - 0xD800) << 10) + (low.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch = char::from_u32(code).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err("a JSON string with a wrong escape".into()),
                    }
                }
                c => out.push(c),
            }
        }
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let t = self.s.get(self.i..self.i + 4).ok_or("a JSON string with a wrong escape")?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(t).map_err(|_| "a JSON string with a wrong escape")?, 16)
            .map_err(|_| "a JSON string with a wrong escape".into())
    }
}
