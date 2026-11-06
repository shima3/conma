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
    /// The native code of the module (vm/native), if the binary has it.
    pub native: RefCell<Option<Rc<crate::native::Table>>>,
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
    /// load_module compiles them (the numbers of vm/native/translate.py).
    pub func_id: Option<usize>,
    pub sinfo: Option<SInfo>,
    pub operator: Option<CExpr>,
    pub olist: Vec<CExpr>,
    pub lcont: Option<CLCont>,
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
    Rc::new(BodyDef { node: node.clone(), func_id, sinfo, operator, olist, lcont })
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
    load_module_native(module, None)
}

/// load_module, and if the binary has native code for the module whose file
/// has this hash (vm/native), attach it to the GVEnv.
pub fn load_module_native(module: &Sexp, hash: Option<u64>) -> Result<Rc<GVEnv>, String> {
    REGISTRY.with(|r| *r.borrow_mut() = Some(Vec::new()));
    let result = load_module_inner(module);
    let funcs = REGISTRY.with(|r| r.borrow_mut().take()).unwrap_or_default();
    let gvenv = result?;
    if let Some(h) = hash {
        let funcs: Vec<Rc<FuncDef>> = funcs.into_iter().flatten().collect();
        let table = crate::native::table_for(h, funcs);
        if std::env::var_os("CONMA_NATIVE_REPORT").is_some() {
            eprintln!("native code: {}", if table.is_some() { "used" } else { "none for this module" });
        }
        *gvenv.native.borrow_mut() = table;
    }
    Ok(gvenv)
}

fn load_module_inner(module: &Sexp) -> Result<Rc<GVEnv>, String> {
    let gvenv = crate::gc::new_gvenv(GVEnv { vals: RefCell::new(Vec::new()), native: RefCell::new(None) });
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
    *gvenv.vals.borrow_mut() = vals;
    Ok(gvenv)
}
