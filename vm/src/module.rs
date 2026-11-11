//! Loading a module (the output of bin/resolver) and compiling its Functions
//! and Bodies once, as `load_module`, `compile_body` and `closure_from_function`
//! in test/vproc_step.py do.

use crate::prim;
use crate::sexp::Sexp;
use crate::value::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The global variables of a module, by GID. None is an undefined variable
/// (Undefined and not a primitive).
pub struct GVEnv {
    pub vals: RefCell<Vec<Option<Value>>>,
    /// The transpiled code of the module (vm/conma2rust), if the binary has it.
    pub transpiled: RefCell<Option<Rc<crate::transpiled::Table>>>,
}

impl GVEnv {
    pub fn get(&self, gid: usize) -> Option<Value> {
        self.vals.borrow().get(gid).cloned().flatten()
    }
}

/// A Function AST, compiled.
pub struct FuncDef {
    pub params: Rc<[Rc<str>]>,
    pub variadic: bool,
    pub body: Option<Rc<BodyDef>>,
}

/// An expression of a Body, compiled.
pub enum CExpr {
    /// A local variable by de Bruijn index.
    L(usize),
    /// A global variable by GID, with its name for error messages.
    G(usize, Rc<str>),
    /// A constant: a String node or null.
    C(Value),
    /// A function expression.
    F(Rc<FuncDef>),
    /// Anything else: evaluating it is an error.
    X(Sexp),
}

pub enum CLCont {
    F(Rc<FuncDef>),
    /// An LCont written as a value (`__Closure__`); not supported.
    V(Sexp),
}

/// A Body AST, compiled. The node is kept for traces.
pub struct BodyDef {
    pub node: Sexp,
    /// The number of the Function whose Body this is, in the order in which
    /// load_module compiles them (the numbers of vm/conma2rust/conma2rust.py).
    pub func_id: Option<usize>,
    pub sinfo: Option<SInfo>,
    pub operator: Option<CExpr>,
    pub olist: Vec<CExpr>,
    pub lcont: Option<CLCont>,
    /// For a Body that applies __Dict_lookup_or__ or __Dict_put__ with a String key written in
    /// place: the key and the cache of this place (step.rs puts it in VProc::dict_site).
    pub dict_site: Option<Rc<DictSite>>,
    /// FAST_LAZY_FORCE for the Body of __LazyList_force__ of include/util.se (mark_fast),
    /// 0 otherwise.
    pub fast: std::cell::Cell<u8>,
    /// IF_THEN_ELSE, IF or UNLESS for a Body that applies __if_then_else__, __if__ or
    /// __unless__ to its operands with each clause written as a function without parameters
    /// (step.rs, load_if), 0 otherwise.
    pub if_site: u8,
    /// For a Body that applies one of INLINE_PRIMS, a global variable, with an LCont written as
    /// a function with parameters: the primitive may be run while the Body is loaded
    /// (step.rs, load_prim).
    pub prim_site: bool,
}

/// The primitives that only pass values to their LCont (taking it from VProc::lcont), apply
/// their onError keeping the LCont, or stop the VProc with an error: step.rs (load_prim) may
/// run them while loading the Body that applies them.
pub const INLINE_PRIMS: &[&str] = &[
    "__Cont_get__",
    "__Dict_lookup_or__",
    "__Dict_put__",
    "__is_null__",
    "__type_name__",
    "__LList_cons__",
    "__LList_uncons__",
    "__Ref_new__",
    "__Ref_get__",
    "__String_equal__",
    "__String_substring__",
    "__String_find__",
    "__String_strip_suffix__",
    "__String_length__",
    "__String_concat__",
    "__String_toCode__",
    "__Number_fromString__",
    "__Number_add__",
    "__Number_subtract__",
    "__Number_is_zero__",
    "__Number_is_negative__",
    "__Number_is_positive__",
];

fn prim_site_of(operator: &Option<CExpr>, lcont: &Option<CLCont>) -> bool {
    matches!(operator, Some(CExpr::G(_, name)) if INLINE_PRIMS.contains(&unquote(name)))
        && matches!(lcont, Some(CLCont::F(f)) if !f.variadic && f.body.is_some())
}

pub const IF_THEN_ELSE: u8 = 1;
pub const IF: u8 = 2;
pub const UNLESS: u8 = 3;

/// The kind of if_site of a Body (BodyDef::if_site).
fn if_site_of(operator: &Option<CExpr>, olist: &[CExpr]) -> u8 {
    let kind = match operator {
        Some(CExpr::G(_, name)) => match unquote(name) {
            "__if_then_else__" => IF_THEN_ELSE,
            "__if__" => IF,
            "__unless__" => UNLESS,
            _ => return 0,
        },
        _ => return 0,
    };
    let n = if kind == IF_THEN_ELSE { 3 } else { 2 };
    let clause = |e: &CExpr| matches!(e, CExpr::F(f) if f.params.is_empty() && !f.variadic && f.body.is_some());
    if olist.len() == n && olist[1..].iter().all(clause) {
        kind
    } else {
        0
    }
}

/// A place that reads or writes a Dict with a String key written in place, for the VM as the
/// Ic of transpiled code is for transpiled code: the key node (a String constant of the OList),
/// its text and str_hash, and the last shape and position found here.
pub struct DictSite {
    pub node: Sexp,
    pub text: Box<str>,
    pub hash: u64,
    pub ic: Ic,
}

/// The DictSite of a Body whose Operator is the global __Dict_lookup_or__ or __Dict_put__ and
/// whose second operand is a String constant without escapes.
fn dict_site_of(operator: &Option<CExpr>, olist: &[CExpr]) -> Option<Rc<DictSite>> {
    match operator {
        Some(CExpr::G(_, name)) if matches!(unquote(name), "__Dict_lookup_or__" | "__Dict_put__") && olist.len() == 3 => {}
        _ => return None,
    }
    let CExpr::C(Value::StrNode(node)) = &olist[1] else { return None };
    let text = node.as_list().and_then(|l| l.get(2)).and_then(|x| x.as_atom()).map(unquote)?;
    if text.contains('\\') {
        return None;
    }
    Some(Rc::new(DictSite { node: node.clone(), text: text.into(), hash: str_hash(text), ic: Ic::new() }))
}

pub fn is_variadic_head(head: Option<&Sexp>) -> bool {
    match head.and_then(|h| h.as_list()) {
        Some(l) => l.iter().skip(2).any(|x| x.as_atom() == Some("Rest")),
        None => false,
    }
}

thread_local! {
    /// The Functions compiled by the load_module running now, by number.
    static REGISTRY: RefCell<Option<Vec<Option<Rc<FuncDef>>>>> = const { RefCell::new(None) };
}

pub fn compile_function(node: &Sexp) -> Rc<FuncDef> {
    let id = REGISTRY.with(|r| {
        r.borrow_mut().as_mut().map(|v| {
            v.push(None);
            v.len() - 1
        })
    });
    let mut head = None;
    let mut body = None;
    if let Some(l) = node.as_list() {
        for child in l.iter().skip(2) {
            match child.tag() {
                Some("Head") => head = Some(child),
                Some("Body") => body = Some(child),
                _ => {}
            }
        }
    }
    let params: Vec<Rc<str>> = match head.and_then(|h| h.as_list()) {
        Some(l) => l
            .iter()
            .skip(2)
            .filter(|c| c.tag() == Some("Variable"))
            .filter_map(|c| c.as_list().and_then(|v| v.get(2)).and_then(|a| a.as_atom()))
            .map(|a| Rc::from(unquote(a)))
            .collect(),
        None => Vec::new(),
    };
    let f = Rc::new(FuncDef {
        params: Rc::from(params),
        variadic: is_variadic_head(head),
        body: body.map(|b| compile_body_of(b, id)),
    });
    if let Some(i) = id {
        REGISTRY.with(|r| {
            if let Some(v) = r.borrow_mut().as_mut() {
                v[i] = Some(f.clone());
            }
        });
    }
    f
}

fn compile_expr(expr: &Sexp) -> CExpr {
    let l = match expr.as_list() {
        Some(l) if !l.is_empty() => l,
        _ => return CExpr::X(expr.clone()),
    };
    match l[0].as_atom() {
        Some("Variable") => {
            let name: Rc<str> = Rc::from(l.get(2).and_then(|a| a.as_atom()).unwrap_or(""));
            if let Some(b) = l.get(3).and_then(|b| b.as_list()) {
                let idx = b.get(1).and_then(|a| a.as_atom()).and_then(|a| a.parse::<usize>().ok());
                match (b.first().and_then(|t| t.as_atom()), idx) {
                    (Some("L"), Some(i)) => return CExpr::L(i),
                    (Some("G"), Some(i)) => return CExpr::G(i, name),
                    _ => {}
                }
            }
            CExpr::X(expr.clone())
        }
        Some("String") => CExpr::C(Value::StrNode(expr.clone())),
        Some("Null") => CExpr::C(Value::Null),
        Some("FuncExp") => match l.get(2) {
            Some(f) => CExpr::F(compile_function(f)),
            None => CExpr::X(expr.clone()),
        },
        _ => CExpr::X(expr.clone()),
    }
}

pub fn compile_body(node: &Sexp) -> Rc<BodyDef> {
    compile_body_of(node, None)
}

fn compile_body_of(node: &Sexp, func_id: Option<usize>) -> Rc<BodyDef> {
    let mut sinfo = None;
    let mut operator = None;
    let mut olist = Vec::new();
    let mut lcont = None;
    if let Some(l) = node.as_list() {
        for child in l.iter().skip(2) {
            let c = match child.as_list() {
                Some(c) => c,
                None => continue,
            };
            match child.tag() {
                Some("SInfo") => sinfo = Some(Rc::new(c[2..].to_vec())),
                Some("Operator") if c.len() > 2 => operator = Some(compile_expr(&c[2])),
                Some("OList") => olist = c[2..].iter().map(compile_expr).collect(),
                Some("LCont") if c.len() > 2 => {
                    let lc = &c[2];
                    lcont = match lc.tag() {
                        Some("Function") => Some(CLCont::F(compile_function(lc))),
                        Some("__Closure__") => Some(CLCont::V(lc.clone())),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
    }
    let dict_site = dict_site_of(&operator, &olist);
    let if_site = if_site_of(&operator, &olist);
    let prim_site = prim_site_of(&operator, &lcont);
    Rc::new(BodyDef {
        node: node.clone(),
        func_id,
        sinfo,
        operator,
        olist,
        lcont,
        dict_site,
        fast: std::cell::Cell::new(0),
        if_site,
        prim_site,
    })
}

/// A Closure of a compiled Function in an environment.
pub fn make_closure(f: &Rc<FuncDef>, lvenv: &LVEnv, gvenv: Option<&Rc<GVEnv>>) -> Value {
    Value::Closure(Rc::new(Closure {
        params: f.params.clone(),
        skip: 0,
        body: f.body.clone(),
        lvenv: lvenv.clone(),
        gvenv: gvenv.cloned(),
        variadic: f.variadic,
    }))
}

/// Map each global name of a module to its GID.
pub fn build_name_index(module: &Sexp) -> HashMap<String, usize> {
    let mut index = HashMap::new();
    if let Some(l) = module.as_list() {
        for item in l.iter().skip(2) {
            if item.tag() != Some("G") {
                continue;
            }
            let il = item.as_list().unwrap();
            let gid = il.get(1).and_then(|a| a.as_atom()).and_then(|a| a.parse::<usize>().ok());
            let name = il.get(2).and_then(|a| a.as_atom()).map(unquote);
            if let (Some(g), Some(n)) = (gid, name) {
                if !n.is_empty() {
                    index.insert(n.to_string(), g);
                }
            }
        }
    }
    index
}

pub fn load_module(module: &Sexp) -> Result<Rc<GVEnv>, String> {
    load_module_transpiled(module, None)
}

/// load_module, and if the binary has transpiled code for the module whose file
/// has this hash (vm/conma2rust), attach it to the GVEnv.
pub fn load_module_transpiled(module: &Sexp, hash: Option<u64>) -> Result<Rc<GVEnv>, String> {
    REGISTRY.with(|r| *r.borrow_mut() = Some(Vec::new()));
    let result = load_module_inner(module);
    let funcs = REGISTRY.with(|r| r.borrow_mut().take()).unwrap_or_default();
    let gvenv = result?;
    if let Some(h) = hash {
        let funcs: Vec<Rc<FuncDef>> = funcs.into_iter().flatten().collect();
        let table = crate::transpiled::table_for(h, funcs);
        if table.is_none() && crate::transpiled::stale_for(h, &module_files(module)) {
            eprintln!(
                "conma: warning: the transpiled code of this program was made from another version of its module, \
                 so it is not used; build the transpiled VM again (vm/conma2rust/build.sh, bench/build.sh)"
            );
        }
        if std::env::var_os("CONMA_TRANSPILED_REPORT").is_some() {
            eprintln!("transpiled code: {}", if table.is_some() { "used" } else { "none for this module" });
        }
        *gvenv.transpiled.borrow_mut() = table;
    }
    Ok(gvenv)
}

/// The files of a module, the atoms of its (F ...) as written, separated by
/// spaces (as module_files of vm/conma2rust/conma2rust.py makes them), or "".
pub fn module_files(module: &Sexp) -> String {
    let f = match module.as_list().and_then(|l| l.get(1)) {
        Some(f) if f.tag() == Some("F") => f,
        _ => return String::new(),
    };
    let atoms: Vec<&str> = f.as_list().unwrap().iter().skip(1).filter_map(|a| a.as_atom()).collect();
    atoms.join(" ")
}

/// The features of fast ConMa that this VM runs (docs/FAST-PLAN-ja.md 3.4):
/// none yet, so a module may have only an empty (Uses).
pub const FAST_FEATURES: &[&str] = &[];

/// The features of fast ConMa that a module uses: the items of its (Uses ...)
/// items, as written.
pub fn fast_features(module: &Sexp) -> Vec<String> {
    let mut features = Vec::new();
    if let Some(l) = module.as_list() {
        for item in l.iter().skip(2) {
            if item.tag() == Some("Uses") {
                features.extend(item.as_list().unwrap().iter().skip(1).map(crate::sexp::to_string));
            }
        }
    }
    features
}

/// An error for the first feature of fast ConMa the module uses that this VM
/// does not run (the message of check_concise of test/vproc_step.py).
fn check_features(module: &Sexp) -> Result<(), String> {
    match fast_features(module).into_iter().find(|f| !FAST_FEATURES.contains(&unquote(f))) {
        Some(f) => Err(format!(
            "the module uses a feature of fast ConMa that this VM does not run: {} \
             (bin/concise translates the module into concise ConMa)",
            f
        )),
        None => Ok(()),
    }
}

fn load_module_inner(module: &Sexp) -> Result<Rc<GVEnv>, String> {
    check_features(module)?;
    let gvenv = crate::gc::new_gvenv(GVEnv { vals: RefCell::new(Vec::new()), transpiled: RefCell::new(None) });
    let mut vals: Vec<Option<Value>> = Vec::new();
    if let Some(l) = module.as_list() {
        for item in l.iter().skip(2) {
            if item.tag() != Some("G") {
                continue;
            }
            let il = item.as_list().unwrap();
            let gid = match il.get(1).and_then(|a| a.as_atom()).and_then(|a| a.parse::<usize>().ok()) {
                Some(g) => g,
                None => return Err(format!("bad GID in {}", crate::sexp::to_string(item))),
            };
            let name = il.get(2).and_then(|a| a.as_atom()).map(unquote).unwrap_or("");
            let val = match il.get(3) {
                None => prim::lookup(name).map(Value::Prim),
                Some(v) if v.as_atom() == Some("Undefined") => prim::lookup(name).map(Value::Prim),
                Some(v) => match v.tag() {
                    Some("Function") => Some(make_closure(&compile_function(v), &None, Some(&gvenv))),
                    Some("Null") => Some(Value::Null),
                    Some("PrimFunc") => {
                        let pname = v.as_list().unwrap().get(1).and_then(|a| a.as_atom()).map(unquote).unwrap_or("");
                        match prim::lookup_pyname(pname) {
                            Some(p) => Some(Value::Prim(p)),
                            None => return Err(format!("Primitive not implemented: {}", pname)),
                        }
                    }
                    _ => Some(Value::Raw(v.clone())),
                },
            };
            if vals.len() <= gid {
                vals.resize(gid + 1, None);
            }
            vals[gid] = val;
        }
    }
    mark_fast(module, &vals);
    *gvenv.vals.borrow_mut() = vals;
    Ok(gvenv)
}

/// BodyDef::fast of the Body of __LazyList_force__ of include/util.se.
pub const FAST_LAZY_FORCE: u8 = 1;

/// The steps from applying __LazyList_force__ of include/util.se to a LazyList that has
/// been evaluated, with an LCont, to applying the LCont to its cell (test/fast2).
pub const LAZY_FORCE_STEPS: u64 = 16;

/// The text of a Function without its positions, SInfo and GIDs (canon).
const LAZY_FORCE_TEXT: &str = r#"(Function (Head (Variable "lazyList" (L 0))) (Body (Operator (Variable "__LazyList_force_claim__" (G))) (OList (Variable "lazyList" (L 0))) (LCont (Function (Head (Variable "cell" (L 0))) (Body (Operator (Null)) (OList (Variable "cell" (L 0))) (LCont))))))"#;
const LAZY_FORCE_CLAIM_TEXT: &str = r#"(Function (Head (Variable "lazyList" (L 0))) (Body (Operator (Variable "__LazyList_claim__" (G))) (OList (Variable "__LazyList_error__" (G)) (Variable "lazyList" (L 0))) (LCont (Function (Head (Variable "thunk" (L 1)) (Variable "cell" (L 0))) (Body (Operator (Variable "__is_null__" (G))) (OList (Variable "thunk" (L 1))) (LCont (Function (Head (Variable "condition" (L 0))) (Body (Operator (Variable "__if_then_else__" (G))) (OList (Variable "condition" (L 0)) (FuncExp (Function (Head) (Body (Operator (Null)) (OList (Variable "cell" (L 1))) (LCont)))) (FuncExp (Function (Head) (Body (Operator (Variable "thunk" (L 2))) (OList) (LCont (Function (Head (Variable "result" (L 0))) (Body (Operator (Variable "__LazyList_force__" (G))) (OList (Variable "result" (L 0))) (LCont (Function (Head (Variable "cell" (L 0))) (Body (Operator (Variable "__LazyList_publish__" (G))) (OList (Variable "lazyList" (L 5)) (Variable "cell" (L 0))) (LCont (Function (Head) (Body (Operator (Null)) (OList (Variable "cell" (L 0))) (LCont)))))))))))))) (LCont)))))))))"#;

/// The text of a node without what depends on where it is: positions, SInfo,
/// and the GIDs of global variables (their names stay).
fn canon(x: &Sexp, out: &mut String) {
    match x.as_list() {
        None => out.push_str(x.as_atom().unwrap_or("")),
        Some(l) => {
            out.push('(');
            let mut first = true;
            for (i, c) in l.iter().enumerate() {
                if let Some(cl) = c.as_list() {
                    let pos = cl.len() == 2 && cl.iter().all(|a| a.as_atom().map_or(false, |t| t.parse::<i64>().is_ok()));
                    if pos || c.tag() == Some("SInfo") {
                        continue;
                    }
                    if c.tag() == Some("G") && i > 0 {
                        if !first {
                            out.push(' ');
                        }
                        out.push_str("(G)");
                        first = false;
                        continue;
                    }
                }
                if !first {
                    out.push(' ');
                }
                canon(c, out);
                first = false;
            }
            out.push(')');
        }
    }
}

/// Mark the Bodies that the VM runs in fewer steps (docs/FAST-PLAN-ja.md 4.2):
/// __LazyList_force__, if it and __LazyList_force_claim__ are the definitions of
/// include/util.se and __LazyList_claim__, __is_null__ and __if_then_else__ are
/// primitives, so that its steps are those LAZY_FORCE_STEPS counts.
fn mark_fast(module: &Sexp, vals: &[Option<Value>]) {
    let l = match module.as_list() {
        Some(l) => l,
        None => return,
    };
    let mut force = None;
    let mut claim_ok = false;
    let mut prims = 0;
    for item in l.iter().skip(2) {
        if item.tag() != Some("G") {
            continue;
        }
        let il = item.as_list().unwrap();
        let name = il.get(2).and_then(|a| a.as_atom()).map(unquote).unwrap_or("");
        let gid = il.get(1).and_then(|a| a.as_atom()).and_then(|a| a.parse::<usize>().ok());
        let text = || {
            let mut t = String::new();
            if let Some(f) = il.get(3) {
                canon(f, &mut t);
            }
            t
        };
        match name {
            "__LazyList_force__" => {
                if text() == LAZY_FORCE_TEXT {
                    force = gid;
                }
            }
            "__LazyList_force_claim__" => {
                claim_ok = text() == LAZY_FORCE_CLAIM_TEXT;
            }
            "__LazyList_claim__" | "__is_null__" | "__if_then_else__" => {
                if matches!(gid.and_then(|g| vals.get(g)).and_then(|v| v.as_ref()), Some(Value::Prim(_))) {
                    prims += 1;
                }
            }
            _ => {}
        }
    }
    let mut marked = false;
    if let (Some(g), true, 3) = (force, claim_ok, prims) {
        if let Some(Some(Value::Closure(c))) = vals.get(g) {
            if let Some(b) = &c.body {
                b.fast.set(FAST_LAZY_FORCE);
                marked = true;
            }
        }
    }
    if std::env::var_os("CONMA_FAST_REPORT").is_some() {
        eprintln!("fast __LazyList_force__: {}", if marked { "used" } else { "none" });
        let count = |p: &dyn Fn(&BodyDef) -> bool| {
            REGISTRY.with(|r| {
                r.borrow().as_ref().map_or(0, |v| v.iter().flatten().filter(|f| f.body.as_deref().map_or(false, p)).count())
            })
        };
        eprintln!("fast __if_then_else__, __if__, __unless__: {} Bodies", count(&|b| b.if_site != 0));
        eprintln!("fast primitives with an LCont: {} Bodies", count(&|b| b.prim_site));
    }
}
