//! Values of ConSh: a String or a LazyList, whose elements are values too
//! (nested LazyLists). A LazyList is evaluated as it is read, at most once
//! for each cell, and may be read from several threads.

use std::sync::{Arc, Condvar, Mutex};

/// How the evaluation of a statement ends other than normally.
#[derive(Debug)]
pub enum Flow {
    /// An error of the script in interactive mode (the message is written).
    Error,
    /// exit: the shell ends with this status.
    Exit(i32),
    /// return from the function call with this id.
    Return(u64),
    /// break and continue of the loop with this id.
    Break(u64),
    Continue(u64),
    /// Stop the command or the thread with this id (a channel whose reader
    /// has cancelled, a variable list that is full: Sh_abort).
    Abort(u64),
    /// The thread was stopped by kill.
    Killed,
}

pub type R<T> = Result<T, Flow>;

#[derive(Clone)]
pub enum Value {
    Str(Arc<str>),
    List(LazyList),
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Arc::from(s))
    }
    pub fn string(s: String) -> Value {
        Value::Str(Arc::from(s))
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::List(_) => None,
        }
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::Str(_))
    }
}

type Thunk = Box<dyn FnOnce() -> R<Option<(Value, LazyList)>> + Send>;

enum Cell {
    Nil,
    Cons(Value, LazyList),
    Thunk(Thunk),
    /// Being evaluated by some thread.
    Busy,
}

struct Lazy {
    cell: Mutex<Cell>,
    done: Condvar,
}

#[derive(Clone)]
pub struct LazyList(Arc<Lazy>);

impl LazyList {
    fn make(cell: Cell) -> LazyList {
        LazyList(Arc::new(Lazy { cell: Mutex::new(cell), done: Condvar::new() }))
    }
    pub fn empty() -> LazyList {
        LazyList::make(Cell::Nil)
    }
    pub fn cons(head: Value, tail: LazyList) -> LazyList {
        LazyList::make(Cell::Cons(head, tail))
    }
    /// A LazyList whose first cell is computed by f when it is first read.
    pub fn new(f: impl FnOnce() -> R<Option<(Value, LazyList)>> + Send + 'static) -> LazyList {
        LazyList::make(Cell::Thunk(Box::new(f)))
    }
    pub fn from_vec(items: Vec<Value>) -> LazyList {
        let mut list = LazyList::empty();
        for v in items.into_iter().rev() {
            list = LazyList::cons(v, list);
        }
        list
    }
    /// The first element and the rest, or None at the end.
    pub fn uncons(&self) -> R<Option<(Value, LazyList)>> {
        let mut cell = self.0.cell.lock().unwrap();
        loop {
            match &*cell {
                Cell::Nil => return Ok(None),
                Cell::Cons(h, t) => return Ok(Some((h.clone(), t.clone()))),
                Cell::Busy => cell = self.0.done.wait(cell).unwrap(),
                Cell::Thunk(_) => break,
            }
        }
        let f = match std::mem::replace(&mut *cell, Cell::Busy) {
            Cell::Thunk(f) => f,
            _ => unreachable!(),
        };
        drop(cell);
        let r = f();
        let mut cell = self.0.cell.lock().unwrap();
        *cell = match &r {
            Ok(None) => Cell::Nil,
            Ok(Some((h, t))) => Cell::Cons(h.clone(), t.clone()),
            // The evaluation failed: later readers see the end.
            Err(_) => Cell::Nil,
        };
        self.0.done.notify_all();
        r
    }
    /// All elements (does not end for an endless LazyList).
    pub fn to_vec(&self) -> R<Vec<Value>> {
        let mut out = Vec::new();
        let mut cur = self.clone();
        while let Some((h, t)) = cur.uncons()? {
            out.push(h);
            cur = t;
        }
        Ok(out)
    }
}
