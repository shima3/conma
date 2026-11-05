//! Channels of ConSh (consh_chan.se): pipes and $( ) between functions. Each
//! thread that writes has its own chain, and may write cap elements that the
//! reader has not taken yet before it waits. The reader takes the elements
//! of the chains in turn, as one LazyList, and ends when every chain is
//! closed.
//!
//! The state of a $( ) (Subst_settle) is "blocked" while a writer waits for
//! room, and "done" when the $( ) has finished. When the reader takes an
//! element from the chain of a waiting writer, it makes the state "running"
//! at once, so that the end of a statement waits until the writer has
//! written again, as in the ConMa version, whichever thread runs first.

use crate::value::{LazyList, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

struct Chain {
    queue: VecDeque<Value>,
    closed: bool,
    waiting: bool,
}

struct Inner {
    chains: Vec<Chain>,
    cancelled: bool,
    done: bool,
    /// The chain to try first (the one after the last taken).
    next: usize,
}

pub struct Chan {
    inner: Mutex<Inner>,
    changed: Condvar,
    cap: usize,
}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum State {
    Running,
    Blocked,
    Done,
}

/// A writer: one chain, used by one thread.
pub struct Writer {
    pub chan: Arc<Chan>,
    index: usize,
}

impl Chan {
    pub fn new(cap: usize) -> Arc<Chan> {
        Arc::new(Chan {
            inner: Mutex::new(Inner { chains: Vec::new(), cancelled: false, done: false, next: 0 }),
            changed: Condvar::new(),
            cap,
        })
    }

    /// A new chain and its writer.
    pub fn writer(self: &Arc<Self>) -> Arc<Writer> {
        let mut g = self.inner.lock().unwrap();
        g.chains.push(Chain { queue: VecDeque::new(), closed: false, waiting: false });
        Arc::new(Writer { chan: self.clone(), index: g.chains.len() - 1 })
    }

    /// The reader will not read any more: writers stop at their next element.
    pub fn cancel(&self) {
        self.inner.lock().unwrap().cancelled = true;
        self.changed.notify_all();
    }

    /// The $( ) has finished (all its writers are closed).
    pub fn set_done(&self) {
        self.inner.lock().unwrap().done = true;
        self.changed.notify_all();
    }

    fn state_of(g: &Inner) -> State {
        if g.done {
            State::Done
        } else if g.chains.iter().any(|c| c.waiting) {
            State::Blocked
        } else {
            State::Running
        }
    }

    /// Wait until the state is not "running" and return it.
    pub fn settle(&self) -> State {
        let mut g = self.inner.lock().unwrap();
        loop {
            let s = Chan::state_of(&g);
            if s != State::Running {
                return s;
            }
            g = self.changed.wait(g).unwrap();
        }
    }

    /// The next element, waiting for a writer; None when every chain is closed.
    fn next(&self) -> Option<Value> {
        crate::os::jitter();
        let mut g = self.inner.lock().unwrap();
        loop {
            let n = g.chains.len();
            for k in 0..n {
                let i = (g.next + k) % n;
                if let Some(v) = g.chains[i].queue.pop_front() {
                    g.chains[i].waiting = false;
                    g.next = i + 1;
                    self.changed.notify_all();
                    return Some(v);
                }
            }
            if g.chains.iter().all(|c| c.closed) {
                return None;
            }
            g = self.changed.wait(g).unwrap();
        }
    }

    /// The elements written to the channel, in the order they are taken.
    pub fn list(self: &Arc<Self>) -> LazyList {
        let chan = self.clone();
        LazyList::new(move || Ok(chan.next().map(|v| (v, chan.list()))))
    }
}

impl Writer {
    /// Write an element, waiting while cap elements are not taken. Err if the
    /// reader has cancelled (like SIGPIPE).
    pub fn put(&self, v: Value) -> Result<(), ()> {
        crate::os::jitter();
        let chan = &self.chan;
        let mut g = chan.inner.lock().unwrap();
        loop {
            if g.cancelled || g.chains[self.index].closed {
                g.chains[self.index].waiting = false;
                return Err(());
            }
            if g.chains[self.index].queue.len() < chan.cap {
                let c = &mut g.chains[self.index];
                c.waiting = false;
                c.queue.push_back(v);
                chan.changed.notify_all();
                return Ok(());
            }
            if !g.chains[self.index].waiting {
                g.chains[self.index].waiting = true;
                chan.changed.notify_all();
            }
            g = chan.changed.wait(g).unwrap();
        }
    }

    /// Close the chain (a second time does nothing).
    pub fn close(&self) {
        let mut g = self.chan.inner.lock().unwrap();
        let c = &mut g.chains[self.index];
        if !c.closed {
            c.closed = true;
            c.waiting = false;
            self.chan.changed.notify_all();
        }
    }
}
