//! The state of a Virtual Process (the class VProc of test/vproc_step.py).

use crate::module::GVEnv;
use crate::value::*;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Ready,
    Running,
    Blocked,
    Waiting,
    Finished,
}

pub struct VProc {
    /// The number shown in traces and error messages.
    pub serial: usize,
    pub state: State,
    pub sinfo: Option<SInfo>,
    pub operator: Value,
    pub olist: Vec<Value>,
    pub lcont: Option<Value>,
    pub lvenv: LVEnv,
    pub gvenv_files: Vec<String>,
    /// An LList of CFrames, or null.
    pub cchain: Value,
    pub pdict: Vec<(Value, Value)>,
    /// GVEnv of the program the VProc was started with.
    pub gvenv: Rc<GVEnv>,
}

impl VProc {
    pub fn new(serial: usize, operator: Value, olist: Vec<Value>, source_file: &str, gvenv: Rc<GVEnv>) -> VProc {
        VProc {
            serial,
            state: State::Ready,
            sinfo: None,
            operator,
            olist,
            lcont: None,
            lvenv: None,
            gvenv_files: vec![source_file.to_string()],
            cchain: Value::Null,
            pdict: Vec::new(),
            gvenv,
        }
    }

    /// True when the VProc has reached a normal halting state.
    pub fn is_terminal(&self) -> bool {
        self.operator.is_null() && self.lcont.is_none() && self.cchain.is_null()
    }

    pub fn cchain_push(&mut self, closure: Value, sinfo: Option<SInfo>) {
        let cf = Value::CFrame(Rc::new(CFrame { closure, sinfo }));
        let tail = std::mem::replace(&mut self.cchain, Value::Null);
        self.cchain = Value::LList(Rc::new(LList { head: cf, tail }));
    }

    /// Pop the top CFrame (None if the CChain is empty).
    pub fn cchain_pop(&mut self) -> Option<Rc<CFrame>> {
        let (cf, rest) = cchain_split(&self.cchain)?;
        self.cchain = rest;
        Some(cf)
    }

    /// The standard return of a primitive: the LCont becomes the Operator,
    /// with results as the OList.
    pub fn prim_return(&mut self, results: Vec<Value>) {
        self.operator = self.lcont.take().unwrap_or(Value::Null);
        self.olist = results;
    }

    /// Apply onError to an error message. The LCont is kept.
    pub fn prim_error(&mut self, on_error: Value, message: &str) {
        self.operator = on_error;
        self.olist = vec![string_value(message)];
    }
}

/// The top CFrame of a CChain and the rest of it.
pub fn cchain_split(cc: &Value) -> Option<(Rc<CFrame>, Value)> {
    match cc {
        Value::LList(cell) => match &cell.head {
            Value::CFrame(cf) => Some((cf.clone(), cell.tail.clone())),
            _ => None,
        },
        _ => None,
    }
}

/// The CFrames of a CChain, top first.
pub fn cchain_frames(cc: &Value) -> Vec<Rc<CFrame>> {
    let mut out = Vec::new();
    let mut cur = cc.clone();
    while let Value::LList(cell) = cur {
        if let Value::CFrame(cf) = &cell.head {
            out.push(cf.clone());
        }
        cur = cell.tail.clone();
    }
    out
}
