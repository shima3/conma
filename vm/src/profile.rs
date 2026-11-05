//! Counting steps for CONMA_PROFILE=<file>: each step is counted under the
//! SInfo (file, line, column) of the Body that the VProc is running, and the
//! steps that apply a primitive also under the name of the primitive. The
//! counts are written to the file when the VM ends. Profiling changes nothing
//! else; the counts can be summed by function with test/vm_profile.py.

use crate::value::{SInfo, Value};
use crate::vproc::VProc;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

pub struct Profile {
    path: String,
    by_sinfo: HashMap<usize, (SInfo, u64)>,
    no_sinfo: u64,
    prims: HashMap<&'static str, u64>,
    /// Calls: the steps that load the Body of a Closure, by that Body.
    calls: HashMap<usize, (SInfo, u64)>,
}

impl Profile {
    pub fn from_env() -> Option<Profile> {
        let path = std::env::var("CONMA_PROFILE").ok().filter(|p| !p.is_empty())?;
        Some(Profile { path, by_sinfo: HashMap::new(), no_sinfo: 0, prims: HashMap::new(), calls: HashMap::new() })
    }

    pub fn count(&mut self, vp: &VProc) {
        match &vp.sinfo {
            Some(s) => self.by_sinfo.entry(Rc::as_ptr(s) as usize).or_insert_with(|| (s.clone(), 0)).1 += 1,
            None => self.no_sinfo += 1,
        }
        match &vp.operator {
            Value::Prim(p) => *self.prims.entry(p.name).or_insert(0) += 1,
            Value::Closure(c) if c.variadic || c.params().is_empty() => {
                if let Some(s) = c.body.as_ref().and_then(|b| b.sinfo.as_ref()) {
                    self.calls.entry(Rc::as_ptr(s) as usize).or_insert_with(|| (s.clone(), 0)).1 += 1;
                }
            }
            _ => {}
        }
    }

    /// Write `count<TAB>body<TAB>file<TAB>line<TAB>column` for each Body, the
    /// same with `call` for the calls of each Body, and
    /// `count<TAB>prim<TAB>name` for each primitive.
    pub fn write(&self) {
        let mut out = String::new();
        for (kind, table) in [("body", &self.by_sinfo), ("call", &self.calls)] {
            let mut merged: HashMap<Vec<String>, u64> = HashMap::new();
            for (s, n) in table.values() {
                let key: Vec<String> = s.iter().filter_map(|x| x.as_atom()).map(|a| crate::value::unquote(a).to_string()).collect();
                *merged.entry(key).or_insert(0) += n;
            }
            for (k, n) in merged {
                out.push_str(&format!("{}\t{}\t{}\n", n, kind, k.join("\t")));
            }
        }
        if self.no_sinfo > 0 {
            out.push_str(&format!("{}\tbody\t-\n", self.no_sinfo));
        }
        for (name, n) in &self.prims {
            out.push_str(&format!("{}\tprim\t{}\n", n, name));
        }
        match std::fs::File::create(&self.path) {
            Ok(mut f) => {
                let _ = f.write_all(out.as_bytes());
            }
            Err(e) => eprintln!("CONMA_PROFILE: {}: {}", self.path, e),
        }
    }
}
