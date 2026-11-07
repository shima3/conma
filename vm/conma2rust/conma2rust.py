#!/usr/bin/env python3
"""conma2rust.py — the ConMa-to-Rust transpiler: Rust code for ConMa modules.

Usage:
    python3 vm/conma2rust/conma2rust.py OUT.rs MODULE.module.se ...

Each MODULE is the output of bin/resolver (test/resolver.sh). OUT.rs is
included by vm/src/transpiled.rs when the VM is built with CONMA_TRANSPILED=OUT.rs
(vm/conma2rust/build.sh). A module's transpiled code is used only for a module file
with the same text (its FNV-1a hash).

The Functions of a module are numbered in the order in which load_module in
vm/src/module.rs compiles them. For each Function whose Body can start
running without the VM, a transpiled function is made that calls a helper of
vm/src/transpiled.rs:

* prim_call: the Operator is a primitive of CHAIN (one that only works on its
  operands and returns its results or applies onError), with an LCont written
  in place or none.
* null_pass: the Operator is __NULL__, with an LCont written in place or
  none (a return).
* call: the Operator is a variable that is not a primitive: a call of a
  Closure of the module with transpiled code is run in place, as a Rust call
  (vm/src/transpiled.rs, rust_call) if the LCont is written in place.
* if_call: __if_then_else__, __if__ or __unless__. A condition `(,() p a ...)`
  with a primitive p of PURE is computed in place; clauses `(,() ...)` are
  run in place.

The helpers go on with the transpiled function of the next Body, so a run of such
Bodies takes one step of the VM.

Fused chains (Fuser, below) replace these per-Body functions for the Bodies
where a Function starts: one Rust function runs the Body and the Bodies after
it with their variables in Rust variables (Numbers in f64 where they go only
between Number primitives), and hands the VProc to the per-Body functions
("bail") where it cannot go on.
"""

import re
import sys
from pathlib import Path

# Primitives that only take their operands and either return their results
# to the LCont (prim_return) or apply a value (onError) with the LCont kept.
# They do not use the LCont or the CChain as values, block, or switch VProcs.
CHAIN = {
    "__is_null__", "__type_name__",
    "__LList_cons__", "__LList_uncons__", "__LList_append__", "__LList_length__",
    "__Number_fromString__", "__Number_toString__", "__Number_is_zero__",
    "__Number_is_positive__", "__Number_is_negative__", "__Number_add__",
    "__Number_subtract__", "__Number_multiply__", "__Number_divide__", "__Number_floor__", "__Number_sqrt__",
    "__String_find_any__", "__String_strip_suffix__", "__String_length__",
    "__String_substring__", "__String_find__", "__Sexp_read__", "__String_concat__",
    "__String_toCode__", "__String_fromCode__", "__String_equal__",
    "__Dict_new__", "__Dict_put__", "__Dict_lookup_or__", "__Dict_remove__",
    "__Dict_keys__", "__Dict_copy__",
    "__MSeq_new__", "__MSeq_is_empty__", "__MSeq_uncons__", "__MSeq_set_head__", "__MSeq_append__",
    "__Ref_new__", "__Ref_get__", "__PDict_put__", "__PDict_lookup_or__",
}
# Not __Ref_set__: the VM wakes the VProcs waiting for a change of a Ref
# (__Ref_wait_change__) after each step, so a change that is undone within a
# run of transpiled code would not wake them, as it does in the VM.

# Primitives of CHAIN without side effects that return one value: a condition
# thunk calling one of them is computed in place (and again by the VM if the
# result is not a Boolean).
PURE = {
    "__is_null__", "__type_name__", "__Number_is_zero__", "__Number_is_positive__",
    "__Number_is_negative__", "__String_equal__", "__String_length__",
    "__LList_length__", "__Ref_get__",
}

IFS = {"__if_then_else__": 3, "__if__": 2, "__unless__": 2}

# Primitives called directly (vm/src/transpiled.rs, d_...): the operands are
# evaluated into Rust variables and the results bound without the OList.
# Used when the Body has exactly this many operands.
DIRECT = {
    "__is_null__": ("d_is_null", 1),
    "__type_name__": ("d_type_name", 1),
    "__Number_add__": ("d_number_add", 2),
    "__Number_subtract__": ("d_number_subtract", 2),
    "__Number_multiply__": ("d_number_multiply", 2),
    "__Number_is_zero__": ("d_number_is_zero", 1),
    "__Number_is_positive__": ("d_number_is_positive", 1),
    "__Number_is_negative__": ("d_number_is_negative", 1),
    "__String_equal__": ("d_string_equal", 2),
    "__String_concat__": ("d_string_concat", 2),
    "__LList_cons__": ("d_llist_cons", 2),
    "__LList_uncons__": ("d_llist_uncons", 2),
    "__Dict_lookup_or__": ("d_dict_lookup_or", 3),
    "__Dict_put__": ("d_dict_put", 3),
    "__Number_divide__": ("d_number_divide", 3),
    "__Number_sqrt__": ("d_number_sqrt", 2),
}


# Number primitives that fused chains compute on f64 Rust variables
# (kind, number of operands); the operands are Numbers, after onError.
NUM_OPS = {
    "__Number_add__": ("bin", 2), "__Number_subtract__": ("bin", 2), "__Number_multiply__": ("bin", 2),
    "__Number_divide__": ("div", 3), "__Number_sqrt__": ("sqrt", 2),
    "__Number_is_zero__": ("pred", 1), "__Number_is_positive__": ("pred", 1), "__Number_is_negative__": ("pred", 1),
}
NUM_BIN = {"__Number_add__": "+", "__Number_subtract__": "-", "__Number_multiply__": "*"}
NUM_PRED = {"__Number_is_zero__": "== 0.0", "__Number_is_positive__": "> 0.0", "__Number_is_negative__": "< 0.0"}


# ── S-expressions (as vm/src/sexp.rs reads them) ──

TOKEN = re.compile(r'"(?:[^"\\]|\\.)*"|[()]|[^\s()]+', re.S)


def parse(text):
    stack = [[]]
    for tok in TOKEN.findall(text):
        if tok == "(":
            stack.append([])
        elif tok == ")":
            done = stack.pop()
            stack[-1].append(done)
        else:
            stack[-1].append(tok)
    return stack[0][0]


def unquote(atom):
    return atom[1:-1] if len(atom) >= 2 and atom[0] == '"' and atom[-1] == '"' else atom


def tag(x):
    return x[0] if isinstance(x, list) and x and isinstance(x[0], str) else None


def fnv1a(data):
    h = 0xcbf29ce484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return h


# ── The Functions of a module, as vm/src/module.rs compiles them ──

class Func:
    def __init__(self, fid):
        self.id = fid
        self.params = 0
        self.variadic = False
        self.body = None


class Body:
    def __init__(self):
        self.operator = None   # ("L", i) | ("G", n) | ("Null",) | ("F", fid) | ("C",) | ("X",)
        self.olist = []
        self.lcont = None      # fid, or "value" for an LCont written as a __Closure__ value


class Compiler:
    def __init__(self):
        self.funcs = []

    def function(self, node):
        f = Func(len(self.funcs))
        self.funcs.append(f)
        head = body = None
        for child in node[2:]:
            if tag(child) == "Head":
                head = child
            elif tag(child) == "Body":
                body = child
        if head is not None:
            f.params = sum(1 for c in head[2:] if tag(c) == "Variable")
            f.variadic = any(c == "Rest" for c in head[2:])
        if body is not None:
            f.body = self.body(body)
        return f.id

    def body(self, node):
        b = Body()
        for child in node[2:]:
            if not isinstance(child, list):
                continue
            t = tag(child)
            if t == "Operator" and len(child) > 2:
                b.operator = self.expr(child[2])
            elif t == "OList":
                b.olist = [self.expr(e) for e in child[2:]]
            elif t == "LCont" and len(child) > 2:
                lc = child[2]
                if tag(lc) == "Function":
                    b.lcont = self.function(lc)
                elif tag(lc) == "__Closure__":
                    b.lcont = "value"
        return b

    def expr(self, e):
        t = tag(e)
        if t == "Variable":
            if len(e) > 3 and isinstance(e[3], list) and len(e[3]) == 2:
                kind, idx = e[3]
                if kind in ("L", "G") and idx.isdigit():
                    return (kind, int(idx))
            return ("X",)
        if t == "String":
            return ("C", e[2] if len(e) > 2 and isinstance(e[2], str) else None)
        if t == "Null":
            return ("Null",)
        if t == "FuncExp" and len(e) > 2:
            return ("F", self.function(e[2]))
        return ("X",)


def load(module, prim_names):
    """The Functions of the module and the primitive name of each global
    variable that is a primitive (Undefined, named as a primitive)."""
    comp = Compiler()
    prims = {}
    comp.tops = []
    for item in module[2:]:
        if tag(item) != "G":
            continue
        gid = int(item[1])
        name = unquote(item[2]) if len(item) > 2 else ""
        value = item[3] if len(item) > 3 else None
        if value is None or value == "Undefined":
            if name in prim_names:
                prims[gid] = name
        elif tag(value) == "Function":
            comp.tops.append(comp.function(value))
    return comp.funcs, prims, comp.tops


# ── Code ──

def thunk(funcs, expr):
    """The Function of `(,() ...)`, or None."""
    if expr[0] != "F":
        return None
    f = funcs[expr[1]]
    if f.params == 0 and not f.variadic and f.body is not None:
        return f.id
    return None


def transpiled_call(funcs, prims, f, prim_index):
    """The helper call of the transpiled function of f, or None."""
    b = f.body
    if b is None or b.lcont == "value" or b.operator is None:
        return None
    lc = "None" if b.lcont is None else f"Some({b.lcont})"
    op = b.operator
    if op == ("Null",):
        return f"null_pass(vp, env, gv, t, {f.id}, {lc})"
    if op[0] == "L" or (op[0] == "G" and op[1] not in prims):
        return f"call(m, vp, env, gv, t, {f.id}, {lc})"
    if op[0] != "G":
        return None
    name = prims[op[1]]
    if name in IFS:
        if len(b.olist) != IFS[name]:
            return None
        cond = "Cond::Value"
        cf = thunk(funcs, b.olist[0])
        if cf is not None:
            cb = funcs[cf].body
            if (cb.lcont is None and cb.operator is not None and cb.operator[0] == "G" and cb.operator[1] in prims
                    and prims[cb.operator[1]] in PURE):
                cond = f"Cond::Thunk({cf}, {prim_index(prims[cb.operator[1]])})"

        def clause(i):
            c = thunk(funcs, b.olist[i])
            return f"Clause::Thunk({c})" if c is not None else f"Clause::Value({i})"
        if name == "__if_then_else__":
            then, els = clause(1), clause(2)
        elif name == "__if__":
            then, els = clause(1), "Clause::Nop"
        else:
            then, els = "Clause::Nop", clause(1)
        return f"if_call(m, vp, env, gv, t, {f.id}, {cond}, {then}, {els}, {lc})"
    if name == "__Cont_get__" and b.lcont is not None and not b.olist:
        return (f"let b = body_of(t, {f.id}); vp.sinfo = b.sinfo.clone(); "
                f"let o = cont_get(m, vp)?; finish(vp, env, gv, t, {lc}, o)")
    if name in DIRECT and len(b.olist) == DIRECT[name][1]:
        fn, n = DIRECT[name]
        args = "".join(f"let a{i} = eval(&b.olist[{i}], &env, gv)?; " for i in range(n))
        names = ", ".join(f"&a{i}" for i in range(n))
        return (f"let b = body_of(t, {f.id}); {args}vp.sinfo = b.sinfo.clone(); "
                f"let o = {fn}(m, {names})?; finish(vp, env, gv, t, {lc}, o)")
    if name in CHAIN:
        return f"prim_call(m, vp, env, gv, t, {f.id}, {prim_index(name)}, {lc})"
    return None


# ── Fused chains ──
#
# For a top-level Function, a function expression, and the LConts and thunks
# where the code below hands the VProc back to the code above (a "bail"), one
# Rust function runs the Body and the Bodies after it that it can, keeping
# the variables they bind in Rust variables instead of LVEnv nodes:
# primitives of DIRECT, __NULL__, __if_then_else__, __if__ and __unless__
# without an LCont and with clauses written in place (both are inlined), and
# calls with an LCont (Rust calls, vm/src/transpiled.rs fused_call; the chain
# binds the values they return), and at the end a return or a call without an
# LCont (left to `apply`). At any
# other Body it makes the LVEnv the VM would have (the LVEnv it started with
# and its variables) and enters the code of that Body above ("bail"), so the
# results are those of the VM.

class Fuser:
    def __init__(self, funcs, prims, tops):
        self.funcs, self.prims, self.tops = funcs, prims, tops

    def apply(self, calls):
        heads = list(self.tops)
        for f in self.funcs:   # function expressions given as operands
            if f.body is not None:
                for e in f.body.olist:
                    if e[0] == "F":
                        heads.append(e[1])
        done = set()
        while heads:
            fid = heads.pop()
            if fid in done:
                continue
            done.add(fid)
            f = self.funcs[fid]
            if f.body is None or f.variadic:
                continue
            self.bails = set()
            out = []
            self.emit(fid, 0, [], out)
            for x in self.bails:
                b = self.funcs[x].body
                if isinstance(b.lcont, int):
                    heads.append(b.lcont)
                for e in b.olist:
                    if e[0] == "F":
                        heads.append(e[1])
            if fid in self.bails:
                continue   # its first Body is not fused: keep the code above
            calls[fid] = "\n    ".join(out)

    def local(self, d, locals_, i):
        """The Rust variable of the variable with de Bruijn index i (< d), and
        whether it is an f64."""
        idx = d - 1 - i
        return f"x{idx}", locals_[idx][2] == "f"

    def ref(self, x, d, e, where, locals_):
        """Rust code for the value of expression e of Function x's Body at
        depth d (variables bound in the chain); where is "olist[k]" or
        "operator". None if it needs the LVEnv (a function expression)."""
        k = e[0]
        if k == "L":
            i = e[1]
            if i < d:
                v, is_f = self.local(d, locals_, i)
                return f"num({v})" if is_f else f"{v}.clone()"
            return f"lget(&env, {i - d})?"
        if k == "Null":
            return "Value::Null"
        if k in ("G", "C"):
            src = "b.operator.as_ref().unwrap()" if where == "operator" else f"&b.{where}"
            return f"{{ let b = body_of(t, {x}); eval({src}, &env, gv)? }}"
        return None

    def rref(self, x, d, e, k, locals_):
        """Rust code for a reference to the value of operand k (expression e)
        of Function x's Body at depth d, for a primitive of DIRECT."""
        kind = e[0]
        if kind == "L":
            i = e[1]
            if i < d:
                v, is_f = self.local(d, locals_, i)
                return f"&num({v})" if is_f else f"&{v}"
            return f"&lget(&env, {i - d})?"
        if kind == "Null":
            return "&Value::Null"
        if kind == "C":
            return f"cref(t, {x}, {k})"
        if kind == "G":
            return f"&{{ let b = body_of(t, {x}); eval(&b.olist[{k}], &env, gv)? }}"
        return None

    def fnum(self, x, d, e, k, name, locals_):
        """Rust code for the f64 of operand k of a Number primitive."""
        if e[0] == "L" and e[1] < d:
            v, is_f = self.local(d, locals_, e[1])
            if is_f:
                return v
        r = self.rref(x, d, e, k, locals_)
        return None if r is None else f'chk(vp, t, {x}, "{name}", {r})?'

    def mat(self, d, locals_):
        if d == 0:
            return "env.clone()"
        pushes = "".join(f"let e = lv_push(&e, t.funcs[{lc}].params[{j}].clone(), "
                         f"{f'num(x{i})' if kind == 'f' else f'x{i}.clone()'}); "
                         for i, (lc, j, kind) in enumerate(locals_))
        return f"{{ let e = env.clone(); {pushes}e }}"

    def bail(self, x, d, locals_, out):
        self.bails.add(x)
        out.append(f"return Ok(Next::Enter({x}, {self.mat(d, locals_)}));")

    def sinfo(self, x):
        return f"&body_of(t, {x}).sinfo"

    def emit(self, x, d, locals_, out):
        """The code of Function x's Body at depth d, ending with a return."""
        f = self.funcs[x]
        b = f.body
        if b is None or b.operator is None or b.lcont == "value":
            return self.bail(x, d, locals_, out)
        op = b.operator
        name = self.prims.get(op[1]) if op[0] == "G" else None
        args = [self.ref(x, d, e, f"olist[{k}]", locals_) for k, e in enumerate(b.olist)]
        lc = b.lcont
        g1 = self.funcs[lc] if isinstance(lc, int) else None
        one = g1 is not None and not g1.variadic and g1.params == 1
        if one and name in NUM_OPS and len(b.olist) == NUM_OPS[name][1]:
            kind, n = NUM_OPS[name]
            ops = b.olist[n - (2 if kind in ("bin", "div") else 1):]
            first = n - len(ops)
            fs = [self.fnum(x, d, e, first + k, name, locals_) for k, e in enumerate(ops)]
            oe_ok = kind not in ("div", "sqrt") or self.ref(x, d, b.olist[0], "olist[0]", locals_) is not None
            if None not in fs and oe_ok:
                for k, f_ in enumerate(fs):
                    out.append(f"let n{k}: f64 = {f_};")
                if kind in ("div", "sqrt"):
                    bad = "n1 == 0.0" if kind == "div" else "n0 < 0.0"
                    msg = ("__Number_divide__: division by zero" if kind == "div"
                           else "__Number_sqrt__: square root of a negative number")
                    oe = self.ref(x, d, b.olist[0], "olist[0]", locals_)
                    out.append(f"if {bad} {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); let oe = {oe}; "
                               f"return finish(vp, {self.mat(d, locals_)}, gv, t, Some({lc}), Out::Call(oe, string_value({msg!r}))); }}".replace("'", '"'))
                if kind == "pred":
                    out.append(f"let x{d} = if n0 {NUM_PRED[name]} {{ m.true_closure() }} else {{ m.false_closure() }};")
                    return self.emit(lc, d + 1, locals_ + [(lc, 0, "v")], out)
                expr = {"bin": f"n0 {NUM_BIN.get(name)} n1", "div": "n0 / n1", "sqrt": "n0.sqrt()"}[kind]
                out.append(f"let x{d}: f64 = {expr};")
                return self.emit(lc, d + 1, locals_ + [(lc, 0, "f")], out)
        key = b.olist[1] if name in ("__Dict_lookup_or__", "__Dict_put__") and len(b.olist) == 3 else None
        if (key is not None and key[0] == "C" and key[1] and "\\" not in key[1] and None not in args
                and self.rref(x, d, b.olist[0], 0, locals_) is not None):
            # a String key written in place: its text, without escapes
            text = key[1][1:-1]
            err = f".map_err(|e| {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); e }})?"
            out.append(f"let a0: &Value = {self.rref(x, d, b.olist[0], 0, locals_)};")
            if name == "__Dict_put__":
                out.append(f"let a2: &Value = {self.rref(x, d, b.olist[2], 2, locals_)};")
                out.append(f'd_dict_put_str(t, {x}, a0, "{text}", a2){err};')
                out.append("let o = Out::R0;")
                return self.direct_result(x, d, locals_, lc, out)
            # the value, or onMissing applied to the key (evaluated only then)
            out.append(f'let o = match d_dict_lookup_str(t, {x}, a0, "{text}"){err} {{ Some(v) => Out::R1(v), '
                       f'None => Out::Call({args[2]}, cref(t, {x}, 1).clone()) }};')
            return self.direct_result(x, d, locals_, lc, out)
        if name in DIRECT and len(b.olist) == DIRECT[name][1] and None not in args:
            fn = DIRECT[name][0]
            names = []
            for k, e in enumerate(b.olist):
                out.append(f"let a{k}: &Value = {self.rref(x, d, e, k, locals_)};")
                names.append(f"a{k}")
            out.append(f"let o = {fn}(m, {', '.join(names)}).map_err(|e| {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); e }})?;")
            return self.direct_result(x, d, locals_, lc, out)
        return self.emit_rest(x, d, locals_, out, b, op, name, args, lc)

    def direct_result(self, x, d, locals_, lc, out):
        """After `let o = d_...(...)`: bind the results to the LCont lc."""
        if True:
            if lc is None:
                out.append(f"vp.sinfo = body_of(t, {x}).sinfo.clone(); return finish(vp, env.clone(), gv, t, None, o);")
                return
            g = self.funcs[lc]
            fallback = f"o => {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); return finish(vp, {self.mat(d, locals_)}, gv, t, Some({lc}), o); }}"
            if g.variadic or g.params > 2:
                out.append(f"match o {{ {fallback} }}")
                return
            vs = [f"x{d + j}" for j in range(g.params)]
            pat = ["Out::R0", "Out::R1({})", "Out::R2({}, {})"][g.params].format(*vs)
            tup = "(" + "".join(v + ", " for v in vs) + ")"
            out.append(f"let {tup} = match o {{ {pat} => {tup}, {fallback} }};")
            return self.emit(lc, d + g.params, locals_ + [(lc, j, "v") for j in range(g.params)], out)

    def emit_rest(self, x, d, locals_, out, b, op, name, args, lc):
        if op == ("Null",) and None not in args:
            if lc is None:
                out.append(f"let v = vec![{', '.join(args)}]; return fused_return(vp, env, {self.sinfo(x)}, v);")
                return
            g = self.funcs[lc]
            if g.variadic or g.params != len(args):
                return self.bail(x, d, locals_, out)
            kinds = []
            for j, (a, e) in enumerate(zip(args, b.olist)):
                if e[0] == "L" and e[1] < d and self.local(d, locals_, e[1])[1]:
                    out.append(f"let x{d + j}: f64 = {self.local(d, locals_, e[1])[0]};")
                    kinds.append("f")
                else:
                    out.append(f"let x{d + j} = {a};")
                    kinds.append("v")
            return self.emit(lc, d + g.params, locals_ + [(lc, j, kinds[j]) for j in range(g.params)], out)
        if name in IFS and len(b.olist) == IFS[name] and lc is None and args[0] is not None:
            def clause(i):
                if i is None:
                    return "nop"
                c = thunk(self.funcs, b.olist[i])
                return c if c is not None else "value"
            if name == "__if_then_else__":
                then, els = clause(1), clause(2)
            elif name == "__if__":
                then, els = clause(1), None
            else:
                then, els = None, clause(1)
            then = then if then is not None else "nop"
            els = els if els is not None else "nop"
            if "value" in (then, els):
                return self.bail(x, d, locals_, out)
            out.append(f"let c = {args[0]};")
            out.append("match boolean_choice(&c) {")
            for label, cl in (("Some(true)", then), ("Some(false)", els)):
                out.append(f"{label} => {{")
                if cl == "nop":
                    out.append(f"return fused_return(vp, env, {self.sinfo(x)}, vec![]);")
                else:
                    self.emit(cl, d, locals_, out)
                out.append("}")
            out.append("None => {")
            self.bail(x, d, locals_, out)
            out.append("}")
            out.append("}")
            return
        callable_op = op[0] == "L" or (op[0] == "G" and op[1] not in self.prims)
        f_ref = self.ref(x, d, op, "operator", locals_) if callable_op else None
        if callable_op and isinstance(lc, int) and None not in args and f_ref is not None:
            # a call with an LCont: a Rust call, whose values the chain binds
            g = self.funcs[lc]
            mat = self.mat(d, locals_)
            out.append(f"let f = {f_ref};")
            out.append(f"let v = vec![{', '.join(args)}];")
            out.append(f"let o = fused_call(m, vp, f, v, gv, t, {lc}, &|| {mat}, {self.sinfo(x)})?;")
            if g.variadic:
                out.append(f"match o {{ Called::Values(v) => return fused_pass(vp, gv, t, {lc}, {mat}, v), Called::Next(n) => return Ok(n) }}")
                return
            out.append(f"let v = match o {{ Called::Values(v) if v.len() == {g.params} => v, "
                       f"Called::Values(v) => return fused_pass(vp, gv, t, {lc}, {mat}, v), Called::Next(n) => return Ok(n) }};")
            if g.params:
                out.append("let mut v = v.into_iter();")
                for j in range(g.params):
                    out.append(f"let x{d + j} = v.next().unwrap();")
            return self.emit(lc, d + g.params, locals_ + [(lc, j, "v") for j in range(g.params)], out)
        if callable_op and lc is None and None not in args:
            f_code = self.ref(x, d, op, "operator", locals_)
            out.append(f"let f = {f_code};")
            out.append(f"let v = vec![{', '.join(args)}]; return fused_tail_call(vp, env, {self.sinfo(x)}, f, v);")
            return
        return self.bail(x, d, locals_, out)


def translate(index, path, prim_names, out):
    data = Path(path).read_bytes()
    module = parse(data.decode("utf-8"))
    if tag(module) != "Module":
        raise ValueError("no Module")
    funcs, prims, tops = load(module, prim_names)
    used = []

    def prim_index(name):
        if name not in used:
            used.append(name)
        return used.index(name)
    calls = [transpiled_call(funcs, prims, f, prim_index) for f in funcs]
    Fuser(funcs, prims, tops).apply(calls)
    p = f"m{index}"
    out.append(f"// {path}: {len(funcs)} Functions, {sum(n is not None for n in calls)} with transpiled code")
    for f, call in zip(funcs, calls):
        if call is not None:
            out.append(f"fn {p}_f{f.id}(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> NResult {{ {call} }}")
    entries = ", ".join(f"Some({p}_f{f.id} as TranspiledFn)" if c is not None else "None" for f, c in zip(funcs, calls))
    out.append(f"static {p.upper()}_FNS: [Option<TranspiledFn>; {len(funcs)}] = [{entries}];")
    names = ", ".join(f'"{n}"' for n in used)
    return (f"Module {{ hash: 0x{fnv1a(data):016x}, nfuncs: {len(funcs)}, "
            f"prims: &[{names}], fns: &{p.upper()}_FNS }}")


def prim_names_of_vm():
    src = (Path(__file__).resolve().parent.parent / "src" / "prim" / "mod.rs").read_text()
    return set(re.findall(r'^\s*"(__\w+__)"\s*=>', src, re.M))


def main():
    if len(sys.argv) < 3:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        sys.exit(2)
    prim_names = prim_names_of_vm()
    out = ["// Made by vm/conma2rust/conma2rust.py. Do not edit."]
    modules = []
    for path in sys.argv[2:]:
        try:
            modules.append(translate(len(modules), path, prim_names, out))
        except (IndexError, ValueError, UnicodeDecodeError) as e:
            print(f"conma2rust.py: {path}: not a module ({e!r}); skipped", file=sys.stderr)
    out.append("pub static MODULES: &[Module] = &[" + ", ".join(modules) + "];")
    Path(sys.argv[1]).write_text("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
