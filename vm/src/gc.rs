//! Collecting cycles. Values are reference counted, so a cycle (a Ref, an MSeq,
//! a Dict, a VProc or a GVEnv that holds itself through other values) is never
//! freed by Rc alone. As the cycle collector of Python, this one does not need
//! the roots: it finds every Rc that the mutable containers reach, counts the
//! references to each one from inside that graph, and treats the values that
//! have more references (strong_count) as reachable from outside, that is, from
//! the scheduler, the VProc running a step or any Rust code. The containers
//! that no such value reaches are garbage, and their contents are dropped,
//! which breaks the cycles. A reference that the traversal does not see only
//! keeps a value alive, so the collector errs on the safe side.
//!
//! It runs between two steps, when no RefCell is borrowed, and only after
//! enough containers have been made since the last collection.

use crate::module::GVEnv;
use crate::value::*;
use crate::vproc::VProcCell;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// A mutable container, as the registry holds it.
enum Weakly {
    Ref(Weak<RefObj>),
    MSeq(Weak<MSeqObj>),
    Dict(Weak<DictObj>),
    VProc(Weak<VProcCell>),
    GV(Weak<GVEnv>),
}

thread_local! {
    static REGISTRY: RefCell<Vec<Weakly>> = const { RefCell::new(Vec::new()) };
    /// CONMA_GC_THRESHOLD, which sets the threshold for every collection (a
    /// small one tests the collector).
    static FIXED: Option<usize> = fixed_threshold();
}

// Plain statics rather than thread locals, because due() is called at every
// step. Only the VM thread uses them.
/// Containers registered since the last collection.
static MADE: AtomicUsize = AtomicUsize::new(0);
/// MADE that starts the next collection.
static THRESHOLD: AtomicUsize = AtomicUsize::new(MIN_THRESHOLD);

/// Read CONMA_GC_THRESHOLD; called before the VM runs.
pub fn init() {
    if let Some(n) = FIXED.with(|f| *f) {
        THRESHOLD.store(n, Relaxed);
    }
}

/// The fewest containers made between two collections. After a collection,
/// the threshold is the number of containers that survived it, so that the
/// time spent collecting stays proportional to the time spent running.
const MIN_THRESHOLD: usize = 10_000;

fn fixed_threshold() -> Option<usize> {
    std::env::var("CONMA_GC_THRESHOLD").ok().and_then(|s| s.parse().ok())
}

fn register(w: Weakly) {
    REGISTRY.with(|r| r.borrow_mut().push(w));
    MADE.fetch_add(1, Relaxed);
}

pub fn new_ref(r: RefObj) -> Rc<RefObj> {
    let rc = Rc::new(r);
    register(Weakly::Ref(Rc::downgrade(&rc)));
    rc
}

pub fn new_mseq(m: MSeqObj) -> Rc<MSeqObj> {
    let rc = Rc::new(m);
    register(Weakly::MSeq(Rc::downgrade(&rc)));
    rc
}

pub fn new_dict(d: DictObj) -> Rc<DictObj> {
    let rc = Rc::new(d);
    register(Weakly::Dict(Rc::downgrade(&rc)));
    rc
}

pub fn new_vproc(v: VProcCell) -> Rc<VProcCell> {
    let rc = Rc::new(v);
    register(Weakly::VProc(Rc::downgrade(&rc)));
    rc
}

pub fn new_gvenv(g: GVEnv) -> Rc<GVEnv> {
    let rc = Rc::new(g);
    register(Weakly::GV(Rc::downgrade(&rc)));
    rc
}

/// True when enough containers have been made to collect.
pub fn due() -> bool {
    MADE.load(Relaxed) >= THRESHOLD.load(Relaxed)
}

/// A value that may be on a cycle, held while collecting.
enum Node {
    Closure(Rc<Closure>),
    Seq(Rc<SeqClosure>),
    CFrame(Rc<CFrame>),
    LList(Rc<LList>),
    Cont(Rc<ContObj>),
    LV(Rc<LVNode>),
    Ref(Rc<RefObj>),
    MSeq(Rc<MSeqObj>),
    Dict(Rc<DictObj>),
    VProc(Rc<VProcCell>),
    GV(Rc<GVEnv>),
    /// The items of MSeqs, which the rests of __MSeq_uncons__ share.
    Items(Rc<Vec<Value>>),
    /// The entries of Dicts, which copies share (DictObj::copy).
    Entries(Rc<Vec<(Value, Value)>>),
}

impl Node {
    fn addr(&self) -> usize {
        match self {
            Node::Closure(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Seq(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::CFrame(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::LList(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Cont(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::LV(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Ref(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::MSeq(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Dict(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::VProc(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::GV(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Items(r) => Rc::as_ptr(r) as *const u8 as usize,
            Node::Entries(r) => Rc::as_ptr(r) as *const u8 as usize,
        }
    }

    fn strong_count(&self) -> usize {
        match self {
            Node::Closure(r) => Rc::strong_count(r),
            Node::Seq(r) => Rc::strong_count(r),
            Node::CFrame(r) => Rc::strong_count(r),
            Node::LList(r) => Rc::strong_count(r),
            Node::Cont(r) => Rc::strong_count(r),
            Node::LV(r) => Rc::strong_count(r),
            Node::Ref(r) => Rc::strong_count(r),
            Node::MSeq(r) => Rc::strong_count(r),
            Node::Dict(r) => Rc::strong_count(r),
            Node::VProc(r) => Rc::strong_count(r),
            Node::GV(r) => Rc::strong_count(r),
            Node::Items(r) => Rc::strong_count(r),
            Node::Entries(r) => Rc::strong_count(r),
        }
    }

    /// Call f on each child, once for each Rc that this value holds. Returns
    /// false if a RefCell is borrowed, so the children cannot be seen.
    fn children(&self, f: &mut dyn FnMut(Node)) -> bool {
        match self {
            Node::Closure(c) => {
                lvenv_child(&c.lvenv, f);
                if let Some(g) = &c.gvenv {
                    f(Node::GV(g.clone()));
                }
            }
            Node::Seq(s) => {
                s.olist.iter().for_each(|v| value_child(v, f));
                lvenv_child(&s.lvenv, f);
            }
            Node::CFrame(cf) => value_child(&cf.closure, f),
            Node::LList(l) => {
                value_child(&l.head, f);
                value_child(&l.tail, f);
            }
            Node::Cont(c) => {
                let Ok(cc) = c.cchain.try_borrow() else { return false };
                value_child(&cc, f)
            }
            Node::LV(n) => {
                value_child(&n.value, f);
                lvenv_child(&n.next, f);
            }
            Node::Ref(r) => {
                let (Ok(v), Ok(lazy)) = (r.value.try_borrow(), r.lazy.try_borrow()) else { return false };
                value_child(&v, f);
                if let Some(info) = &*lazy {
                    if let Some(c) = &info.chain {
                        value_child(c, f);
                    }
                    value_child(&info.thunk, f);
                }
            }
            Node::MSeq(m) => {
                let Ok(items) = m.items.try_borrow() else { return false };
                f(Node::Items(items.clone()));
            }
            Node::Items(items) => items.iter().for_each(|v| value_child(v, f)),
            Node::Dict(d) => {
                let Ok(entries) = d.entries.try_borrow() else { return false };
                f(Node::Entries(entries.clone()));
            }
            Node::Entries(e) => {
                for (k, v) in e.iter() {
                    value_child(k, f);
                    value_child(v, f);
                }
            }
            Node::VProc(cell) => {
                let Ok(vp) = cell.vp.try_borrow() else { return false };
                value_child(&vp.operator, f);
                vp.olist.iter().for_each(|v| value_child(v, f));
                if let Some(l) = &vp.lcont {
                    value_child(l, f);
                }
                lvenv_child(&vp.lvenv, f);
                value_child(&vp.cchain, f);
                for (k, v) in vp.pdict.iter() {
                    value_child(k, f);
                    value_child(v, f);
                }
                f(Node::GV(vp.gvenv.clone()));
                for r in vp.claims.iter() {
                    f(Node::Ref(r.clone()));
                }
            }
            Node::GV(g) => {
                let Ok(vals) = g.vals.try_borrow() else { return false };
                vals.iter().flatten().for_each(|v| value_child(v, f));
            }
        }
        true
    }

    fn is_container(&self) -> bool {
        matches!(self, Node::Ref(_) | Node::MSeq(_) | Node::Dict(_) | Node::VProc(_) | Node::GV(_))
    }

    /// Take the contents out of a container that is garbage.
    fn clear(&self, trash: &mut Vec<Value>, lv_trash: &mut Vec<LVEnv>, entries_trash: &mut Vec<Rc<Vec<(Value, Value)>>>, items_trash: &mut Vec<Rc<Vec<Value>>>) {
        match self {
            Node::Ref(r) => {
                trash.push(std::mem::replace(&mut *r.value.borrow_mut(), Value::Null));
                if let Some(info) = r.lazy.borrow_mut().take() {
                    trash.extend(info.chain);
                    trash.push(info.thunk);
                }
            }
            // The items go when no MSeq shares them any more (another garbage
            // MSeq may still share them until it is cleared).
            Node::MSeq(m) => items_trash.push(std::mem::take(&mut *m.items.borrow_mut())),
            Node::Dict(d) => {
                // The entries go when no Dict shares them any more (another
                // garbage Dict may still share them until it is cleared).
                entries_trash.push(std::mem::take(&mut *d.entries.borrow_mut()));
                *d.index.borrow_mut() = Rc::default();
            }
            Node::VProc(cell) => {
                let mut vp = cell.vp.borrow_mut();
                trash.push(std::mem::replace(&mut vp.operator, Value::Null));
                trash.append(&mut vp.olist);
                trash.extend(vp.lcont.take());
                lv_trash.push(vp.lvenv.take());
                trash.push(std::mem::replace(&mut vp.cchain, Value::Null));
                for (k, v) in vp.pdict.drain(..) {
                    trash.push(k);
                    trash.push(v);
                }
                trash.extend(vp.claims.drain(..).map(Value::Ref));
            }
            Node::GV(g) => trash.extend(g.vals.borrow_mut().iter_mut().filter_map(|v| v.take())),
            _ => {}
        }
    }
}

fn value_child(v: &Value, f: &mut dyn FnMut(Node)) {
    match v {
        Value::Closure(r) => f(Node::Closure(r.clone())),
        Value::Seq(r) => f(Node::Seq(r.clone())),
        Value::CFrame(r) => f(Node::CFrame(r.clone())),
        Value::LList(r) => f(Node::LList(r.clone())),
        Value::Cont(r) => f(Node::Cont(r.clone())),
        Value::Ref(r) => f(Node::Ref(r.clone())),
        Value::MSeq(r) => f(Node::MSeq(r.clone())),
        Value::Dict(r) => f(Node::Dict(r.clone())),
        Value::VProc(r) => f(Node::VProc(r.clone())),
        // Strings, Numbers, primitives, AST nodes, Streams and OSProcs hold no values.
        _ => {}
    }
}

fn lvenv_child(env: &LVEnv, f: &mut dyn FnMut(Node)) {
    if let Some(n) = env {
        f(Node::LV(n.clone()));
    }
}

/// Collect the cycles that nothing outside them reaches. Returns the number of
/// containers freed.
pub fn collect() -> usize {
    // The containers alive, each held once more by `nodes`.
    let mut nodes: Vec<Node> = Vec::new();
    REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.retain(|w| {
            let n = match w {
                Weakly::Ref(w) => w.upgrade().map(Node::Ref),
                Weakly::MSeq(w) => w.upgrade().map(Node::MSeq),
                Weakly::Dict(w) => w.upgrade().map(Node::Dict),
                Weakly::VProc(w) => w.upgrade().map(Node::VProc),
                Weakly::GV(w) => w.upgrade().map(Node::GV),
            };
            match n {
                Some(n) => {
                    nodes.push(n);
                    true
                }
                None => false,
            }
        });
    });
    let mut index: HashMap<usize, usize> = HashMap::with_capacity(nodes.len() * 4);
    for (i, n) in nodes.iter().enumerate() {
        index.insert(n.addr(), i);
    }
    // Every value reachable from the containers, the references to each one
    // from inside the graph, and the edges.
    let mut internal: Vec<usize> = vec![0; nodes.len()];
    let mut opaque: Vec<bool> = vec![false; nodes.len()];
    let mut edge_start: Vec<usize> = Vec::with_capacity(nodes.len());
    let mut edges: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < nodes.len() {
        edge_start.push(edges.len());
        let mut found: Vec<Node> = Vec::new();
        let seen = nodes[i].children(&mut |c| found.push(c));
        if !seen {
            opaque[i] = true;
        }
        for c in found {
            let j = match index.get(&c.addr()) {
                Some(&j) => j,
                None => {
                    let j = nodes.len();
                    index.insert(c.addr(), j);
                    nodes.push(c);
                    internal.push(0);
                    opaque.push(false);
                    j
                }
            };
            internal[j] += 1;
            edges.push(j);
        }
        i += 1;
    }
    edge_start.push(edges.len());
    // A value with more references than the graph and `nodes` hold is
    // reachable from outside, and so is everything it reaches.
    let mut live = vec![false; nodes.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (j, n) in nodes.iter().enumerate() {
        // The children were collected into `found` and dropped, so each value
        // is held here once, by `nodes`.
        if opaque[j] || n.strong_count() > internal[j] + 1 {
            live[j] = true;
            stack.push(j);
        }
    }
    while let Some(j) = stack.pop() {
        for &k in &edges[edge_start[j]..edge_start[j + 1]] {
            if !live[k] {
                live[k] = true;
                stack.push(k);
            }
        }
    }
    let mut trash: Vec<Value> = Vec::new();
    let mut lv_trash: Vec<LVEnv> = Vec::new();
    let mut entries_trash: Vec<Rc<Vec<(Value, Value)>>> = Vec::new();
    let mut items_trash: Vec<Rc<Vec<Value>>> = Vec::new();
    let mut freed = 0;
    for (j, n) in nodes.iter().enumerate() {
        if !live[j] && n.is_container() {
            n.clear(&mut trash, &mut lv_trash, &mut entries_trash, &mut items_trash);
            freed += 1;
        }
    }
    let survivors = nodes.iter().zip(&live).filter(|(n, &l)| l && n.is_container()).count();
    // Every value the collection went through, containers or not (LVEnv
    // nodes, CFrames, LLists ...).
    let visited = nodes.len();
    if std::env::var_os("CONMA_GC_STATS").is_some() {
        let mut kinds: HashMap<&str, usize> = HashMap::new();
        for (n, &l) in nodes.iter().zip(&live) {
            if l {
                let k = match n { Node::Closure(_) => "Closure", Node::Seq(_) => "Seq", Node::CFrame(_) => "CFrame", Node::LList(_) => "LList", Node::Cont(_) => "Cont", Node::LV(_) => "LV", Node::Ref(_) => "Ref", Node::MSeq(_) => "MSeq", Node::Dict(_) => "Dict", Node::VProc(_) => "VProc", Node::GV(_) => "GV", Node::Items(_) => "Items", Node::Entries(_) => "Entries" };
                *kinds.entry(k).or_default() += 1;
            }
        }
        let roots = nodes.iter().enumerate().filter(|(j, n)| n.is_container() && n.strong_count() > internal[*j] + 1).count();
        eprintln!("gc: freed {} survivors {} roots {} live {:?}", freed, survivors, roots, kinds);
    }
    drop(nodes);
    drop(index);
    drop(trash);
    drop(lv_trash);
    drop(items_trash);
    drop(entries_trash);
    MADE.store(0, Relaxed);
    // The next collection comes after as many containers as survived, and at
    // least a quarter as many as the values this one went through, so that
    // the time spent collecting stays proportional to the containers made
    // even when few containers reach many other values (a deep CChain).
    let next = FIXED.with(|f| *f).unwrap_or(survivors.max(MIN_THRESHOLD).max(visited / 4));
    THRESHOLD.store(next, Relaxed);
    freed
}
