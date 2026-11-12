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
    /// Calls by caller: the Body running when a Body is loaded, and that Body.
    edges: HashMap<(usize, usize), (Option<SInfo>, SInfo, u64)>,
    /// Conditions that are not Booleans (written as functions), passed to
    /// __if_then_else__, __if__ or __unless__, by the Body that passes them.
    thunks: HashMap<usize, (SInfo, u64)>,
    /// Applications of Closures and SeqClosures, by kind (see `apply_kind`)
    /// and by the Body that applies them.
    apps: HashMap<(&'static str, usize), (Option<SInfo>, u64)>,
}

/// The kind of the step that applies the operator of `vp`, or None when the
/// operator is not a Closure or a SeqClosure, or the step only loads the Body
/// of a Closure whose parameters are all bound (the second step of an application):
/// "exact" as many operands as parameters (the Closure not partially applied),
/// "exact-rest" the same for a partially applied Closure, "partial" fewer,
/// "over" more, "pass" a partially applied Closure with no operands (passed on),
/// "surplus" a Closure without parameters with operands (they go to a SeqClosure),
/// "variadic" a variadic Closure, "seq" a SeqClosure.
fn apply_kind(vp: &VProc) -> Option<&'static str> {
    match &vp.operator {
        Value::Seq(_) => Some("seq"),
        Value::Closure(c) if c.variadic => Some("variadic"),
        Value::Closure(c) => {
            let (p, o) = (c.params().len(), vp.olist.len());
            Some(match (p, o) {
                (0, 0) => return None,
                (0, _) => "surplus",
                (_, 0) => "pass",
                _ if o < p => "partial",
                _ if o > p => "over",
                _ if c.skip > 0 => "exact-rest",
                _ => "exact",
            })
        }
        _ => None,
    }
}

impl Profile {
    pub fn from_env() -> Option<Profile> {
        let path = std::env::var("CONMA_PROFILE").ok().filter(|p| !p.is_empty())?;
        Some(Profile { path, by_sinfo: HashMap::new(), no_sinfo: 0, prims: HashMap::new(), calls: HashMap::new(), edges: HashMap::new(), thunks: HashMap::new(), apps: HashMap::new() })
    }

    pub fn count(&mut self, vp: &VProc) {
        if let Some(kind) = apply_kind(vp) {
            let at = vp.sinfo.as_ref().map(|s| Rc::as_ptr(s) as usize).unwrap_or(0);
            self.apps.entry((kind, at)).or_insert_with(|| (vp.sinfo.clone(), 0)).1 += 1;
        }
        match &vp.sinfo {
            Some(s) => self.by_sinfo.entry(Rc::as_ptr(s) as usize).or_insert_with(|| (s.clone(), 0)).1 += 1,
            None => self.no_sinfo += 1,
        }
        match &vp.operator {
            Value::Prim(p) => {
                *self.prims.entry(p.name).or_insert(0) += 1;
                if matches!(p.name, "__if_then_else__" | "__if__" | "__unless__") {
                    if let (Some(cond), Some(s)) = (vp.olist.first(), &vp.sinfo) {
                        if crate::prim::control::boolean_choice(cond).is_none() {
                            self.thunks.entry(Rc::as_ptr(s) as usize).or_insert_with(|| (s.clone(), 0)).1 += 1;
                        }
                    }
                }
            }
            Value::Closure(c) if c.variadic || c.params().is_empty() => {
                if let Some(s) = c.body.as_ref().and_then(|b| b.sinfo.as_ref()) {
                    self.calls.entry(Rc::as_ptr(s) as usize).or_insert_with(|| (s.clone(), 0)).1 += 1;
                    let caller = vp.sinfo.as_ref().map(|c| Rc::as_ptr(c) as usize).unwrap_or(0);
                    self.edges
                        .entry((caller, Rc::as_ptr(s) as usize))
                        .or_insert_with(|| (vp.sinfo.clone(), s.clone(), 0))
                        .2 += 1;
                }
            }
            _ => {}
        }
    }

    /// Write `count<TAB>body<TAB>file<TAB>line<TAB>column` for each Body, the
    /// same with `call` for the calls of each Body and with `thunk` for the
    /// conditions that are not Booleans, by the Body that passes them,
    /// `count<TAB>edge<TAB>caller file, line, column<TAB>callee file, line, column`
    /// for the calls by caller,
    /// `count<TAB>apply<TAB>kind<TAB>file<TAB>line<TAB>column` for the applications
    /// of each kind, by the Body that applies them, and
    /// `count<TAB>prim<TAB>name` for each primitive.
    pub fn write(&self) {
        let mut out = String::new();
        for (kind, table) in [("body", &self.by_sinfo), ("call", &self.calls), ("thunk", &self.thunks)] {
            let mut merged: HashMap<Vec<String>, u64> = HashMap::new();
            for (s, n) in table.values() {
                // Only the file, line and column, as format_error (the SJS translator adds "sjs" as a 4th string)
                let key: Vec<String> = s.iter().filter_map(|x| x.as_atom()).take(3).map(|a| crate::value::unquote(a).to_string()).collect();
                *merged.entry(key).or_insert(0) += n;
            }
            for (k, n) in merged {
                out.push_str(&format!("{}\t{}\t{}\n", n, kind, k.join("\t")));
            }
        }
        let atoms = |s: &SInfo| -> String {
            s.iter().filter_map(|x| x.as_atom()).take(3).map(|a| crate::value::unquote(a).to_string()).collect::<Vec<_>>().join("\t")
        };
        let mut merged: HashMap<(String, String), u64> = HashMap::new();
        for (caller, callee, n) in self.edges.values() {
            let from = caller.as_ref().map(atoms).unwrap_or_else(|| "-\t0\t0".to_string());
            *merged.entry((from, atoms(callee))).or_insert(0) += n;
        }
        for ((from, to), n) in merged {
            out.push_str(&format!("{}\tedge\t{}\t{}\n", n, from, to));
        }
        let mut merged: HashMap<(&str, String), u64> = HashMap::new();
        for ((kind, _), (at, n)) in &self.apps {
            let at = at.as_ref().map(atoms).unwrap_or_else(|| "-\t0\t0".to_string());
            *merged.entry((kind, at)).or_insert(0) += n;
        }
        for ((kind, at), n) in merged {
            out.push_str(&format!("{}\tapply\t{}\t{}\n", n, kind, at));
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
