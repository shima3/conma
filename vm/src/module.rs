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
    /// A variable of the frame at the head of the LVEnv, by slot (frame_chain).
    S(usize),
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
    /// IF_THEN_ELSE, IF, UNLESS, IF_NULL_THEN_ELSE, UNLESS_NULL, IF_ZERO_THEN_ELSE, UNLESS_ZERO
    /// IF_GREATER or IF_GREATER_THEN_ELSE for a Body that applies __if_then_else__, __if__,
    /// __unless__, __if_null_then_else__, __unless_null__, __Number_if_zero_then_else__, __Number_unless_zero__,
    /// __Number_if_greater__ or __Number_if_greater_then_else__ to its operands with each clause written as a function without parameters
    /// (step.rs, load_if), 0 otherwise.
    pub if_site: u8,
    /// For a Body that applies one of INLINE_PRIMS, a global variable, with an LCont written as
    /// a function with parameters: the primitive may be run while the Body is loaded
    /// (step.rs, load_prim).
    pub prim_site: bool,
    /// For a Body `__Number_fromString__ onError "text" ,(x) ...` whose text is a Number (an
    /// experiment, docs/SPEED-ja.md): the Body of the LCont with the Number in place of x and the
    /// variables beyond it one nearer, made when it is first needed (const_fold); step.rs
    /// (load_const) loads it in place of the three steps, when the Operator is the primitive.
    pub fold_site: bool,
    pub folded: std::cell::OnceCell<Option<Rc<BodyDef>>>,
    /// How the values passed to the Function whose Body this is are bound (frame_chain,
    /// value::lv_bind): 0, a node for each; 1, a new frame node; 2, into the frame node.
    pub frame: u8,
    /// For frame 1: the variables the frame will hold.
    pub frame_size: u32,
    /// The names of the variables of the LVEnv the Body runs in (its own and those of the
    /// Functions around it), for traces and printed Closures.
    pub scope: Scope,
}

/// The names of the variables of an LVEnv, by the Functions that bind them, innermost first:
/// LVEnv nodes keep no names (value::LVNode). Each node is a Function's parameters in order.
pub type Scope = Option<Rc<ScopeNode>>;

pub struct ScopeNode {
    pub names: Rc<[Rc<str>]>,
    pub next: Scope,
}

/// The scope of a Body of a Function with these parameters, in the scope around it.
pub fn scope_push(names: &Rc<[Rc<str>]>, next: &Scope) -> Scope {
    Some(Rc::new(ScopeNode { names: names.clone(), next: next.clone() }))
}

/// The name of the variable with de Bruijn index i in scope.
pub fn scope_name(scope: &Scope, mut i: usize) -> Option<Rc<str>> {
    let mut cur = scope.as_ref();
    while let Some(s) = cur {
        let n = s.names.len();
        if i < n {
            return Some(s.names[n - 1 - i].clone());
        }
        i -= n;
        cur = s.next.as_ref();
    }
    None
}

static NAMES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Keep the scope of the LVEnv of each VProc and SeqClosure (VProc::scope), for traces
/// (main.rs, with --trace).
pub fn keep_scopes() {
    NAMES.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[inline]
pub fn scopes_kept() -> bool {
    NAMES.load(std::sync::atomic::Ordering::Relaxed)
}

/// The primitives that only pass values to their LCont (taking it from VProc::lcont), apply
/// their onError keeping the LCont, or stop the VProc with an error: step.rs (load_prim) may
/// run them while loading the Body that applies them.
pub const INLINE_PRIMS: &[&str] = &[
    "__Cont_get__",
    "__Dict_lookup_or__",
    "__TRUE__",
    "__FALSE__",
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
    "__Number_is_negative_zero__",
    "__Number_and32__",
    "__Number_or32__",
    "__Number_xor32__",
    "__Number_not32__",
    "__Number_shl32__",
    "__Number_shr32__",
    "__Number_ushr32__",
];

fn prim_site_of(operator: &Option<CExpr>, lcont: &Option<CLCont>) -> bool {
    matches!(operator, Some(CExpr::G(_, name)) if INLINE_PRIMS.contains(&unquote(name)))
        && matches!(lcont, Some(CLCont::F(f)) if !f.variadic && f.body.is_some())
}

pub const IF_THEN_ELSE: u8 = 1;
pub const IF: u8 = 2;
pub const UNLESS: u8 = 3;
pub const IF_NULL_THEN_ELSE: u8 = 4;
pub const UNLESS_NULL: u8 = 5;
pub const IF_ZERO_THEN_ELSE: u8 = 6;
pub const UNLESS_ZERO: u8 = 7;
pub const IF_GREATER: u8 = 8;
pub const IF_GREATER_THEN_ELSE: u8 = 9;

/// The kind of if_site of a Body (BodyDef::if_site).
fn if_site_of(operator: &Option<CExpr>, olist: &[CExpr]) -> u8 {
    let kind = match operator {
        Some(CExpr::G(_, name)) => match unquote(name) {
            "__if_then_else__" => IF_THEN_ELSE,
            "__if__" => IF,
            "__unless__" => UNLESS,
            "__if_null_then_else__" => IF_NULL_THEN_ELSE,
            "__unless_null__" => UNLESS_NULL,
            "__Number_if_zero_then_else__" => IF_ZERO_THEN_ELSE,
            "__Number_unless_zero__" => UNLESS_ZERO,
            "__Number_if_greater__" => IF_GREATER,
            "__Number_if_greater_then_else__" => IF_GREATER_THEN_ELSE,
            _ => return 0,
        },
        _ => return 0,
    };
    let n = match kind {
        IF_GREATER_THEN_ELSE => 4,
        IF_THEN_ELSE | IF_NULL_THEN_ELSE | IF_ZERO_THEN_ELSE | IF_GREATER => 3,
        _ => 2,
    };
    // the operands that are clauses (__Number_if_greater__ and __Number_if_greater_then_else__
    // compare the first two)
    let first_clause = if matches!(kind, IF_GREATER | IF_GREATER_THEN_ELSE) { 2 } else { 1 };
    let clause = |e: &CExpr| matches!(e, CExpr::F(f) if f.params.is_empty() && !f.variadic && f.body.is_some());
    if olist.len() == n && olist[first_clause..].iter().all(clause) {
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
    compile_function_in(node, &None)
}

/// A Function in the scope around it.
fn compile_function_in(node: &Sexp, outer: &Scope) -> Rc<FuncDef> {
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
    let params: Rc<[Rc<str>]> = Rc::from(params);
    let scope = scope_push(&params, outer);
    let f = Rc::new(FuncDef {
        params,
        variadic: is_variadic_head(head),
        body: body.map(|b| compile_body_of(b, id, &scope)),
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

fn compile_expr(expr: &Sexp, scope: &Scope) -> CExpr {
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
            Some(f) => CExpr::F(frame_chain(compile_function_in(f, scope))),
            None => CExpr::X(expr.clone()),
        },
        _ => CExpr::X(expr.clone()),
    }
}

/// A Body run in an LVEnv of variables with these names (the parameters of its Function).
pub fn compile_body(node: &Sexp, params: &[&str]) -> Rc<BodyDef> {
    let names: Rc<[Rc<str>]> = params.iter().map(|p| Rc::from(*p)).collect();
    compile_body_of(node, None, &scope_push(&names, &None))
}

fn compile_body_of(node: &Sexp, func_id: Option<usize>, scope: &Scope) -> Rc<BodyDef> {
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
                Some("Operator") if c.len() > 2 => operator = Some(compile_expr(&c[2], scope)),
                Some("OList") => olist = c[2..].iter().map(|e| compile_expr(e, scope)).collect(),
                Some("LCont") if c.len() > 2 => {
                    let lc = &c[2];
                    lcont = match lc.tag() {
                        Some("Function") => Some(CLCont::F(compile_function_in(lc, scope))),
                        Some("__Closure__") => Some(CLCont::V(lc.clone())),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
    }
    new_body(node.clone(), func_id, sinfo, operator, olist, lcont, scope.clone())
}

fn new_body(
    node: Sexp,
    func_id: Option<usize>,
    sinfo: Option<SInfo>,
    operator: Option<CExpr>,
    olist: Vec<CExpr>,
    lcont: Option<CLCont>,
    scope: Scope,
) -> Rc<BodyDef> {
    new_body_frame(node, func_id, sinfo, operator, olist, lcont, scope, (0, 0))
}

#[allow(clippy::too_many_arguments)]
fn new_body_frame(
    node: Sexp,
    func_id: Option<usize>,
    sinfo: Option<SInfo>,
    operator: Option<CExpr>,
    olist: Vec<CExpr>,
    lcont: Option<CLCont>,
    scope: Scope,
    frame: (u8, u32),
) -> Rc<BodyDef> {
    let dict_site = dict_site_of(&operator, &olist);
    let if_site = if_site_of(&operator, &olist);
    let prim_site = prim_site_of(&operator, &lcont);
    let fold_site = fold_number(&operator, &olist, &lcont).is_some();
    Rc::new(BodyDef {
        node,
        func_id,
        sinfo,
        operator,
        olist,
        lcont,
        dict_site,
        fast: std::cell::Cell::new(0),
        if_site,
        prim_site,
        fold_site,
        folded: std::cell::OnceCell::new(),
        scope,
        frame: frame.0,
        frame_size: frame.1,
    })
}

/// The Number of a Body `__Number_fromString__ onError "text" ,(x) ...` whose text is a
/// Number that __Number_fromString__ returns, and the Function of its LCont.
fn fold_number<'a>(operator: &Option<CExpr>, olist: &[CExpr], lcont: &'a Option<CLCont>) -> Option<(f64, &'a Rc<FuncDef>)> {
    match operator {
        Some(CExpr::G(_, name)) if unquote(name) == "__Number_fromString__" => {}
        _ => return None,
    }
    let f = match lcont {
        Some(CLCont::F(f)) if f.params.len() == 1 && !f.variadic && f.body.is_some() => f,
        _ => return None,
    };
    let text = match olist {
        [_, CExpr::C(v @ Value::StrNode(_))] => crate::trace::value_to_display(v).ok()?,
        _ => return None,
    };
    if !prim::number::is_number_syntax(&text) {
        return None;
    }
    let n = prim::number::parse_number(&text);
    if n.is_infinite() {
        return None;
    }
    Some((n, f))
}

/// The Body of the LCont of a fold_site Body with its Number in place (BodyDef::folded).
pub fn const_fold(b: &BodyDef) -> Option<Rc<BodyDef>> {
    b.folded
        .get_or_init(|| {
            let (n, f) = fold_number(&b.operator, &b.olist, &b.lcont)?;
            // Not into a frame chain (frame_chain), whose variables are not numbered so.
            if f.body.as_ref()?.frame != 0 {
                return None;
            }
            subst_body(f.body.as_ref()?, 0, &num(n), &b.scope)
        })
        .clone()
}

/// e with the variable at de Bruijn index off replaced by v, and those beyond it one nearer.
fn subst_expr(e: &CExpr, off: usize, v: &Value, scope: &Scope) -> Option<CExpr> {
    Some(match e {
        CExpr::L(i) if *i == off => CExpr::C(v.clone()),
        CExpr::L(i) if *i > off => CExpr::L(i - 1),
        CExpr::L(i) => CExpr::L(*i),
        CExpr::S(_) => return None,
        CExpr::G(g, name) => CExpr::G(*g, name.clone()),
        CExpr::C(c) => CExpr::C(c.clone()),
        CExpr::F(f) => CExpr::F(subst_func(f, off, v, scope)?),
        CExpr::X(_) => return None,
    })
}

/// scope: the scope around f once the variable is gone.
fn subst_func(f: &Rc<FuncDef>, off: usize, v: &Value, scope: &Scope) -> Option<Rc<FuncDef>> {
    let body = match &f.body {
        Some(b) => Some(subst_body(b, off + f.params.len(), v, &scope_push(&f.params, scope))?),
        None => None,
    };
    Some(Rc::new(FuncDef { params: f.params.clone(), variadic: f.variadic, body }))
}

/// scope: the scope of the new Body (that of b without the variable).
fn subst_body(b: &BodyDef, off: usize, v: &Value, scope: &Scope) -> Option<Rc<BodyDef>> {
    // Not into a frame chain (frame_chain) inside the LCont either: its variables beyond the frame
    // are counted with the frame as one node, and new_body would drop the frame.
    if b.frame != 0 {
        return None;
    }
    let operator = match &b.operator {
        Some(e) => Some(subst_expr(e, off, v, scope)?),
        None => None,
    };
    let olist = b.olist.iter().map(|e| subst_expr(e, off, v, scope)).collect::<Option<Vec<_>>>()?;
    let lcont = match &b.lcont {
        Some(CLCont::F(f)) => Some(CLCont::F(subst_func(f, off, v, scope)?)),
        Some(CLCont::V(_)) => return None,
        None => None,
    };
    Some(new_body(b.node.clone(), None, b.sinfo.clone(), operator, olist, lcont, scope.clone()))
}

/// The fewest variables that the LConts of a chain bind for frame_chain to bind them into a
/// frame (CONMA_FRAME_MIN=n sets it; an empty or bad value, no frames).
const FRAME_MIN_DEFAULT: usize = 8;

thread_local! {
    static FRAME_MIN: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
}

/// f, or, when its Body and the Bodies of the LConts after it (written as Functions, the chain)
/// make no Closure (no function expression among their Operators and operands) and the LConts
/// bind at least FRAME_MIN variables, f with the variables of these LConts bound into one frame
/// node of the LVEnv (value::lv_bind) instead of a node each: the first LCont with parameters makes
/// it (BodyDef::frame 1) and the next ones add theirs (2), and in each Body a variable of the
/// frame is read by slot (CExpr::S) and one beyond it by its index counting the frame as one node.
/// The parameters of f itself are bound as before. A Function nested in another is given to this
/// when it is compiled (compile_expr), before the Function around it.
fn frame_chain(f: Rc<FuncDef>) -> Rc<FuncDef> {
    let min = FRAME_MIN.with(|c| c.get());
    if min == usize::MAX {
        return f;
    }
    let mut bodies: Vec<Rc<BodyDef>> = Vec::new();
    let mut funcs: Vec<Rc<FuncDef>> = Vec::new();
    let mut cur = match &f.body {
        Some(b) => b.clone(),
        None => return f,
    };
    loop {
        let makes_closure = cur.operator.iter().chain(cur.olist.iter()).any(|e| matches!(e, CExpr::F(_) | CExpr::S(_)));
        if makes_closure {
            return f;
        }
        bodies.push(cur.clone());
        match &cur.lcont {
            Some(CLCont::F(g)) if !g.variadic && g.body.is_some() => {
                funcs.push(g.clone());
                cur = g.body.clone().unwrap();
            }
            // An LCont whose Body would not be rewritten.
            Some(_) => return f,
            None => break,
        }
    }
    let total: usize = funcs.iter().map(|g| g.params.len()).sum();
    if total < min {
        return f;
    }
    // m[j]: the variables in the frame when bodies[j] runs.
    let mut m = vec![0usize; bodies.len()];
    for j in 1..bodies.len() {
        m[j] = m[j - 1] + funcs[j - 1].params.len();
    }
    let map = |e: &CExpr, m: usize| -> CExpr {
        match e {
            CExpr::L(i) if m > 0 && *i < m => CExpr::S(m - 1 - *i),
            CExpr::L(i) if m > 0 => CExpr::L(*i - m + 1),
            CExpr::L(i) => CExpr::L(*i),
            CExpr::G(g, n) => CExpr::G(*g, n.clone()),
            CExpr::C(c) => CExpr::C(c.clone()),
            CExpr::X(x) => CExpr::X(x.clone()),
            CExpr::S(_) | CExpr::F(_) => unreachable!(),
        }
    };
    // Rebuild from the last Body.
    let mut next: Option<CLCont> = None;
    for j in (0..bodies.len()).rev() {
        let b = &bodies[j];
        let frame = if j == 0 || funcs[j - 1].params.is_empty() {
            0
        } else if m[j - 1] == 0 {
            1
        } else {
            2
        };
        let lcont = next.take();
        let nb = new_body_frame(
            b.node.clone(),
            b.func_id,
            b.sinfo.clone(),
            b.operator.as_ref().map(|e| map(e, m[j])),
            b.olist.iter().map(|e| map(e, m[j])).collect(),
            lcont,
            b.scope.clone(),
            (frame, total as u32),
        );
        nb.fast.set(b.fast.get());
        if j == 0 {
            return Rc::new(FuncDef { params: f.params.clone(), variadic: f.variadic, body: Some(nb) });
        }
        let g = &funcs[j - 1];
        next = Some(CLCont::F(Rc::new(FuncDef { params: g.params.clone(), variadic: false, body: Some(nb) })));
    }
    unreachable!()
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
    // Frames only where the LVEnvs are not those of transpiled code or of traces.
    FRAME_MIN.with(|c| {
        c.set(if hash.is_some_and(crate::transpiled::has_code) || scopes_kept() {
            usize::MAX
        } else {
            match std::env::var("CONMA_FRAME_MIN") {
                Ok(v) => v.parse().unwrap_or(usize::MAX),
                Err(_) => FRAME_MIN_DEFAULT,
            }
        })
    });
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

/// The features of fast ConMa that this VM runs (docs/FAST-PLAN-ja.md 3.4): "bitwise32", the
/// bitwise primitives __Number_and32__ and the others (docs/BITWISE-PLAN-ja.md, prim/number.rs),
/// "boolean", the primitives __TRUE__ and __FALSE__, and "control", __if_then_else__, __if__,
/// __unless__, __if_null_then_else__, __unless_null__, __Number_if_zero_then_else__, __Number_unless_zero__,
/// __Number_if_greater__, __Number_if_greater_then_else__ and __Cont_get__ (prim/control.rs,
/// prim/cchain.rs), and "wasm", __Wasm_compile__ and __Wasm_call__ (prim/wasm.rs; the native VM
/// has them and fails as it has no WebAssembly).
pub const FAST_FEATURES: &[&str] = &["bitwise32", "boolean", "control", "wasm"];

/// True if the module uses the feature "boolean" of fast ConMa: __TRUE__ and
/// __FALSE__ are primitives (prim/control.rs), and so are the Booleans the
/// primitives return (sched.rs, Machine::use_prim_booleans).
pub fn uses_boolean(module: &Sexp) -> bool {
    fast_features(module).iter().any(|f| unquote(f) == "boolean")
}

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
                    Some("Function") => Some(make_closure(&frame_chain(compile_function(v)), &None, Some(&gvenv))),
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
        // __if_then_else__, __if__, __unless__ and the six conditionals that test a value (load_if)
        eprintln!("fast conditionals: {} Bodies", count(&|b| b.if_site != 0));
        eprintln!("fast primitives with an LCont: {} Bodies", count(&|b| b.prim_site));
    }
}
