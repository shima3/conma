//! Values and VProcs as S-expressions, for traces and for printing values
//! (value_to_sexp and vproc_to_sexp of test/vproc_step.py).

use crate::sexp::Sexp;
use crate::value::*;
use crate::vproc::{cchain_frames, VProc};

fn a(s: &str) -> Sexp {
    Sexp::atom(s)
}
fn l(v: Vec<Sexp>) -> Sexp {
    Sexp::list(v)
}
fn tagged(tag: &str, rest: impl IntoIterator<Item = Sexp>) -> Sexp {
    let mut v = vec![a(tag)];
    v.extend(rest);
    l(v)
}

pub fn null_sexp() -> Sexp {
    l(vec![a("Null"), Sexp::pos0()])
}

fn sinfo_items(s: &Option<SInfo>) -> Vec<Sexp> {
    match s {
        Some(v) => v.as_ref().clone(),
        None => Vec::new(),
    }
}

pub fn value_to_sexp(v: &Value) -> Sexp {
    match v {
        Value::Null => null_sexp(),
        Value::Str(s) => Sexp::Atom(s.clone()),
        Value::StrNode(n) | Value::Raw(n) => n.clone(),
        Value::Number(x) => l(vec![a("__Number__"), a(&number_to_str(*x))]),
        Value::Closure(c) => closure_to_sexp(c),
        Value::Seq(s) => seq_to_sexp(s),
        Value::Prim(p) => l(vec![a("PrimFunc"), a(&quoted(p.pyname))]),
        Value::CFrame(cf) => l(vec![
            a("__CFrame__"),
            value_to_sexp(&cf.closure),
            tagged("__SInfo__", sinfo_items(&cf.sinfo)),
        ]),
        Value::LList(_) => llist_to_sexp(v),
        Value::Ref(r) => l(vec![a("__Ref__"), a(&quoted(&r.serial.to_string()))]),
        Value::MSeq(r) => l(vec![a("__MSeq__"), a(&quoted(&r.serial.to_string()))]),
        Value::Dict(r) => l(vec![a("__Dict__"), a(&quoted(&r.serial.to_string()))]),
        Value::Cont(r) => l(vec![a("__Cont__"), a(&quoted(&r.serial.to_string()))]),
        Value::VProc(r) => l(vec![a("__VProcRef__"), a(&quoted(&r.serial.to_string()))]),
    }
}

// An LList is written as nested (__LList__ head tail); built from the end,
// so that a long list does not recurse.
fn llist_to_sexp(v: &Value) -> Sexp {
    let mut heads = Vec::new();
    let mut cur = v.clone();
    while let Value::LList(cell) = cur {
        heads.push(value_to_sexp(&cell.head));
        cur = cell.tail.clone();
    }
    let mut out = value_to_sexp(&cur);
    for h in heads.into_iter().rev() {
        out = l(vec![a("__LList__"), h, out]);
    }
    out
}

pub fn lvenv_to_sexp(env: &LVEnv) -> Sexp {
    tagged(
        "__LVEnv__",
        lv_iter(env).map(|n| l(vec![a(&quoted(&n.name)), value_to_sexp(&n.value)])),
    )
}

fn closure_to_sexp(c: &Closure) -> Sexp {
    let mut head = vec![a("Head"), Sexp::pos0()];
    if c.variadic {
        head.push(a("Rest"));
    }
    for p in c.params() {
        head.push(l(vec![a("Variable"), Sexp::pos0(), a(&quoted(p))]));
    }
    let body = match &c.body {
        Some(b) => b.node.clone(),
        None => l(vec![a("Body"), Sexp::pos0()]),
    };
    l(vec![a("__Closure__"), l(head), body, lvenv_to_sexp(&c.lvenv)])
}

fn seq_to_sexp(s: &SeqClosure) -> Sexp {
    let mut body = vec![a("Body"), Sexp::pos0()];
    if let Some(si) = &s.sinfo {
        if !si.is_empty() {
            body.push(tagged("__SInfo__", si.as_ref().clone()));
        }
    }
    body.push(l(vec![
        a("Operator"),
        Sexp::pos0(),
        l(vec![a("Variable"), Sexp::pos0(), a("\"__Sink__\""), l(vec![a("L"), a("0")])]),
    ]));
    let mut ol = vec![a("OList"), Sexp::pos0()];
    ol.extend(s.olist.iter().map(value_to_sexp));
    body.push(l(ol));
    body.push(l(vec![a("LCont"), Sexp::pos0(), null_sexp()]));
    l(vec![
        a("__Closure__"),
        l(vec![a("Head"), Sexp::pos0(), l(vec![a("Variable"), Sexp::pos0(), a("\"__Sink__\"")])]),
        l(body),
        lvenv_to_sexp(&s.lvenv),
    ])
}

pub fn vproc_to_sexp(vp: &VProc) -> Sexp {
    let lcont = match &vp.lcont {
        None => l(vec![a("__LCont__")]),
        Some(v) => l(vec![a("__LCont__"), value_to_sexp(v)]),
    };
    let frames = cchain_frames(&vp.cchain)
        .iter()
        .map(|cf| l(vec![value_to_sexp(&cf.closure), tagged("__SInfo__", sinfo_items(&cf.sinfo))]))
        .collect::<Vec<_>>();
    l(vec![
        tagged("__SInfo__", sinfo_items(&vp.sinfo)),
        l(vec![a("__Operator__"), value_to_sexp(&vp.operator)]),
        tagged("__OList__", vp.olist.iter().map(value_to_sexp)),
        lcont,
        lvenv_to_sexp(&vp.lvenv),
        tagged("__GVEnv__", vp.gvenv_files.iter().map(|f| a(&quoted(f)))),
        tagged("__CChain__", frames),
        tagged("__PDict__", vp.pdict.iter().map(|(k, v)| l(vec![value_to_sexp(k), value_to_sexp(v)]))),
    ])
    .prepend_tag("__VProc__")
}

trait Prepend {
    fn prepend_tag(self, tag: &str) -> Sexp;
}
impl Prepend for Sexp {
    fn prepend_tag(self, tag: &str) -> Sexp {
        let mut v = vec![a(tag)];
        if let Some(items) = self.as_list() {
            v.extend(items.iter().cloned());
        }
        l(v)
    }
}

/// A value as __OS_print__ writes it: the text of a String, otherwise its
/// S-expression.
pub fn value_to_display(v: &Value) -> Result<String, String> {
    match string_text(v) {
        Some(r) => r,
        None => Ok(crate::sexp::to_string(&value_to_sexp(v))),
    }
}
