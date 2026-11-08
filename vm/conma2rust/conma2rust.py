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

# Inlining (Fuser.inline): the most Bodies of a Function written in place,
# and the most calls written in place one in another.
INLINE_SIZE = 8
INLINE_DEPTH = 2

# All the variables (Fuser.uses of a Function whose uses are not known).
ALL = None

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
# __Number_floor__ keeps an infinity or NaN as it is (vm/src/prim/number.rs).
NUM_OPS = {
    "__Number_add__": ("bin", 2), "__Number_subtract__": ("bin", 2), "__Number_multiply__": ("bin", 2),
    "__Number_divide__": ("div", 3), "__Number_sqrt__": ("sqrt", 2), "__Number_floor__": ("floor", 1),
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
    gfun = {}
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
            gfun[gid] = comp.function(value)
            comp.tops.append(gfun[gid])
    return comp.funcs, prims, comp.tops, gfun


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
# primitives of DIRECT, __NULL__, __Cont_get__ (the continuation in a Rust
# variable), __if_then_else__, __if__ and __unless__ with clauses written in
# place (both are inlined; with an LCont, in a labeled block whose value is
# what the clause returns, the LCont going on after it), and
# calls with an LCont (Rust calls, vm/src/transpiled.rs fused_call; the chain
# binds the values they return), and at the end a return or a call without an
# LCont (left to `apply`). The parameters of the head are Rust variables
# too: its direct entry (m{i}_d{fid}) takes the arguments as Values, and its
# entry for the VM (m{i}_f{fid}) reads them from the LVEnv and calls it with
# that LVEnv ("full", which a bail reuses). A call of the Closure running without an LCont
# (outside clauses with an LCont) makes the Function a Rust loop: its
# parameters in Rust variables, set to the new arguments (self_call). At any
# other Body it makes the LVEnv the VM would have (the LVEnv it started with
# and its variables) and enters the code of that Body above ("bail"), so the
# results are those of the VM.

class Fuser:
    def __init__(self, funcs, prims, tops, gfun):
        self.funcs, self.prims, self.tops, self.gfun = funcs, prims, tops, gfun
        self.loop = None
        # the code being written: the Rust variable of its LVEnv, the number
        # of the Rust variable of its first chain variable (x{off}), and how
        # many of its first variables are parameters of the head (full)
        self.ctx = {"env": "env", "off": 0, "hp": 0}
        self.bails, self.retries, self.labels = set(), set(), 0
        self.nbail, self.inlining, self.envs = 0, [], 0

    def apply(self, calls, directs):
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
            # its parameters in Rust variables (x0 ...), env the LVEnv of the
            # Closure and full the LVEnv with the parameters if the caller
            # has it; a call of itself without an LCont goes back to the top
            # of a loop
            p = f.params
            self.bails, self.labels, self.loop = set(), 0, {"fid": fid, "p": p, "used": False}
            self.ctx, self.envs = {"env": "env", "off": 0, "hp": p}, 0
            self.retries = set()
            out = []
            self.emit(fid, p, [(fid, j, "v") for j in range(p)], out)
            takes = "".join(f"let mut x{j} = std::mem::replace(&mut a[{j}], Value::Null); " for j in range(p))
            if self.loop["used"]:
                out = ["'l0: loop {"] + out + ["}"]
            out = [f"{takes}let mut full = full;"] + out
            for x in self.bails | self.retries:
                b = self.funcs[x].body
                if isinstance(b.lcont, int):
                    heads.append(b.lcont)
                for e in b.olist:
                    if e[0] == "F":
                        heads.append(e[1])
            if fid in self.bails:
                continue   # its first Body is not fused: keep the code above
            directs[fid] = "\n    ".join(out)
            reads = ", ".join(f"lget(&env, {p - 1 - j})?" for j in range(p))
            calls[fid] = (f"let mut a: [Value; {p}] = [{reads}]; let base = lv_skip(&env, {p}); "
                          f"DIRECT_{fid}(m, vp, base, &mut a, Some(env), gv, t)")

    # ── Which variables of a chain are used later ──
    #
    # uses(fid): the de Bruijn indices (in the LVEnv of the Body of fid) that
    # the Body, its LCont and its function expressions read, or ALL. A
    # variable of the chain that the code after a Body does not read is
    # "dead": its last use moves its value instead of cloning it, and an
    # LVEnv made for the VM has null in its place (the code that runs with
    # that LVEnv does not read it).

    def uses(self, fid):
        memo = self.__dict__.setdefault("_uses", {})
        if fid in memo:
            return memo[fid]
        memo[fid] = ALL   # (a Function does not contain itself)
        b = self.funcs[fid].body
        s = set()
        if b is not None:
            if b.lcont == "value":
                return ALL
            for e in ([b.operator] if b.operator is not None else []) + b.olist:
                u = self.expr_uses(e)
                if u is ALL:
                    return ALL
                s |= u
            if isinstance(b.lcont, int):
                u = self.shifted(b.lcont)
                if u is ALL:
                    return ALL
                s |= u
        memo[fid] = frozenset(s)
        return memo[fid]

    def shifted(self, g):
        """The indices that Function g reads in the LVEnv it is made in."""
        u = self.uses(g)
        if u is ALL:
            return ALL
        p = self.funcs[g].params
        return {i - p for i in u if i >= p}

    def expr_uses(self, e):
        if e[0] == "L":
            return {e[1]}
        if e[0] == "F":
            return self.shifted(e[1])
        if e[0] == "X":
            return ALL
        return set()

    def use_kinds(self, fid, i):
        """How the Body of fid and what it contains use the variable with de
        Bruijn index i (in the LVEnv of the Body): a set of "cond" (as the
        condition of __if_then_else__, __if__ or __unless__) and "other"."""
        b = self.funcs[fid].body
        kinds = set()
        if b is None:
            return kinds
        if b.lcont == "value":
            return {"other"}
        op = b.operator
        name = self.prims.get(op[1]) if op is not None and op[0] == "G" else None
        for k, e in enumerate(([op] if op is not None else []) + b.olist):
            if e[0] == "L" and e[1] == i:
                is_cond = name in IFS and op is not None and k == 1 and len(b.olist) == IFS[name]
                kinds.add("cond" if is_cond else "other")
            elif e[0] == "F":
                kinds |= self.use_kinds(e[1], i + self.funcs[e[1]].params)
            elif e[0] == "X":
                kinds.add("other")
        if isinstance(b.lcont, int):
            kinds |= self.use_kinds(b.lcont, i + self.funcs[b.lcont].params)
        return kinds

    def live(self, x, dx, J):
        """The chain variables (by index) read by Function x's Body run at
        depth dx (no Function: none) and by the code after the clauses J;
        None for all."""
        s = set()
        if x is not None:
            u = self.uses(x)
            if u is ALL:
                return None
            s |= {dx - 1 - i for i in u if i < dx}
        for j in J:
            if j["ctx"] is not self.ctx:
                continue   # a call inlined around it: its variables are not these
            u = self.uses(j["lc"])
            if u is ALL:
                return None
            s |= {j["d"] + j["p"] - 1 - i for i in u if j["p"] <= i < j["d"] + j["p"]}
        return s

    @staticmethod
    def mark(locals_, live):
        """locals_ with the variables not in live (a set; None for all) dead."""
        if live is None:
            return locals_
        return [l if idx in live or l[2] == "d" else (l[0], l[1], "d") for idx, l in enumerate(locals_)]

    def moves(self, d, locals_, exprs, after):
        """For each expression, whether it is the last use of a variable that
        the code after (the indices in after; None for all) does not read:
        its value can be moved."""
        out = [False] * len(exprs)
        if after is None:
            return out
        seen = set()
        for k in range(len(exprs) - 1, -1, -1):
            e = exprs[k]
            if e[0] == "L" and e[1] < d:
                idx = d - 1 - e[1]
                if idx not in after and idx not in seen and locals_[idx][2] == "v":
                    out[k] = True
                seen.add(idx)
        return out

    def local(self, d, locals_, i):
        """The Rust variable of the variable with de Bruijn index i (< d), and
        its kind: "v" a Value, "f" an f64, "b" a bool, "c" a String written in
        place (no Rust variable: cref of its Body and operand, constant)."""
        idx = d - 1 - i
        return self.xn(idx), locals_[idx][2]

    def xn(self, idx):
        """The Rust variable of the chain variable idx of the code being written."""
        return f"x{self.ctx['off'] + idx}"

    def constant(self, x, d, e, k, locals_):
        """For a String written in place (operand k of Function x's Body, or a
        variable bound to one): (Function, operand, its atom); else None."""
        if e[0] == "C" and e[1]:
            return (x, k, e[1])
        if e[0] == "L" and e[1] < d:
            l = locals_[d - 1 - e[1]]
            if l[2] == "c":
                return l[3]
        return None

    def ref(self, x, d, e, where, locals_, move=False):
        """Rust code for the value of expression e of Function x's Body at
        depth d (variables bound in the chain); where is "olist[k]" or
        "operator". None if it needs the LVEnv (a function expression). With
        move, a variable's value is moved out of it."""
        k = e[0]
        E = self.ctx["env"]
        if k == "L":
            i = e[1]
            if i < d:
                v, kind = self.local(d, locals_, i)
                if kind == "f":
                    return f"num({v})"
                if kind == "b":
                    return f"bool_value({v})"
                if kind == "c":
                    cx, ck, _ = locals_[d - 1 - i][3]
                    return f"cref(t, {cx}, {ck}).clone()"
                return v if move else f"{v}.clone()"
            return f"lget(&{E}, {i - d})?"
        if k == "Null":
            return "Value::Null"
        if k in ("G", "C"):
            src = "b.operator.as_ref().unwrap()" if where == "operator" else f"&b.{where}"
            return f"{{ let b = body_of(t, {x}); eval({src}, &{E}, gv)? }}"
        return None

    def rref(self, x, d, e, k, locals_):
        """Rust code for a reference to the value of operand k (expression e)
        of Function x's Body at depth d, for a primitive of DIRECT."""
        kind = e[0]
        E = self.ctx["env"]
        if kind == "L":
            i = e[1]
            if i < d:
                v, vk = self.local(d, locals_, i)
                if vk == "c":
                    cx, ck, _ = locals_[d - 1 - i][3]
                    return f"cref(t, {cx}, {ck})"
                return {"f": f"&num({v})", "b": f"&bool_value({v})"}.get(vk, f"&{v}")
            return f"&lget(&{E}, {i - d})?"
        if kind == "Null":
            return "&Value::Null"
        if kind == "C":
            return f"cref(t, {x}, {k})"
        if kind == "G":
            return f"&{{ let b = body_of(t, {x}); eval(&b.olist[{k}], &{E}, gv)? }}"
        return None

    def fnum(self, x, d, e, k, name, locals_):
        """Rust code for the f64 of operand k of a Number primitive."""
        if e[0] == "L" and e[1] < d:
            v, vk = self.local(d, locals_, e[1])
            if vk == "f":
                return v
        r = self.rref(x, d, e, k, locals_)
        return None if r is None else f'chk(vp, t, {x}, "{name}", {r})?'

    def mat(self, d, locals_, ctx=None):
        """Rust code for the LVEnv the VM would have: env with the variables
        of the chain. The parameters of the head (the first hp) are taken
        from full when it is there. ctx: the code the variables are of (the
        code being written if None)."""
        ctx = ctx or self.ctx
        E, off = ctx["env"], ctx["off"]
        if d == 0:
            return f"{E}.clone()"

        def val(i, l):
            kind = l[2]
            if kind == "c":
                return f"cref(t, {l[3][0]}, {l[3][1]}).clone()"
            x = f"x{off + i}"
            return {"f": f"num({x})", "b": f"bool_value({x})", "d": "Value::Null"}.get(kind, f"{x}.clone()")

        def pushes(items):
            return "".join(f"let e = lv_push(&e, t.funcs[{l[0]}].params[{l[1]}].clone(), {val(i, l)}); "
                           for i, l in items)
        hp = ctx["hp"]
        items = list(enumerate(locals_))
        if hp == 0:
            return f"{{ let e = {E}.clone(); {pushes(items)}e }}"
        return (f"{{ let e = match &full {{ Some(f) => f.clone(), None => {{ let e = {E}.clone(); {pushes(items[:hp])}e }} }}; "
                f"{pushes(items[hp:])}e }}")

    # J, the clauses the code is in: for each __if__ (or __unless__,
    # __if_then_else__) with an LCont whose clause is inlined, outermost
    # first, a dict with its LCont "lc", the depth "d" and variables "locals"
    # of the chain at the __if__, its Function "x", the label "label" of the
    # Rust block whose value the clause gives, and the number "p" of
    # parameters of lc. The VM would have the CFrames of these LConts on the
    # CChain; where the code hands the VProc back (a bail, onError, a call
    # left to the VM), it puts them there first (push_joins), and a Rust call
    # puts them there if it is left (node).

    def wrap(self, J, base):
        """Rust code for the CChain base with the CFrames of the clauses J on
        top (the innermost on top)."""
        for j in J:
            base = (f"cframe_on(t, gv, {j['lc']}, {self.mat(j['d'], j['locals'], j['ctx'])}, "
                    f"&body_of(t, {j['x']}).sinfo, {base})")
        return base

    def push_joins(self, J, out):
        if J:
            out.append(f"vp.cchain = {self.wrap(J, 'std::mem::replace(&mut vp.cchain, Value::Null)')};")

    def node(self, lc, d, locals_, x, J):
        """The CFrame builder of a call with the LCont Function lc."""
        inner = self.wrap(J, "b.clone()")
        if lc is None:
            return f"&|b: &Value| {inner}"
        return f"&|b: &Value| cframe_on(t, gv, {lc}, {self.mat(d, locals_)}, &body_of(t, {x}).sinfo, {inner})"

    def to_join(self, J, vals, x, out):
        """Give the values (Rust code of Values) to the LCont of the innermost
        clause J: the value of its block if they are as many as its
        parameters; otherwise apply its LCont to them as the VM would."""
        j = J[-1]
        if len(vals) == j["p"]:
            tup = "(" + "".join(v + ", " for v in vals) + ")"
            out.append(f"break '{j['label']} {tup};")
        else:
            out.append(f"let v = [{', '.join(vals)}];")
            self.push_joins(J, out)
            out.append(f"return fused_return(m, vp, {self.ctx['env']}, {self.sinfo(x)}, v);")

    def bail(self, x, d, locals_, out, J=(), retry=False, rare=False):
        """Enter the code of Function x's Body above, with the LVEnv the VM
        would have. With retry, x is fused, and its code above is for a case
        the chain leaves to it (not a Body it cannot run). rare: a case that
        is not expected (a condition that is not a Boolean)."""
        (self.retries if retry else self.bails).add(x)
        if not (retry or rare):
            self.nbail += 1
        self.push_joins(J, out)
        out.append(f"return Ok(Next::Enter({x}, {self.mat(d, locals_)}));")

    def sinfo(self, x):
        return f"&body_of(t, {x}).sinfo"

    def emit(self, x, d, locals_, out, J=()):
        """The code of Function x's Body at depth d, ending with a return or
        (in a clause) a break."""
        f = self.funcs[x]
        b = f.body
        locals_ = self.mark(locals_, self.live(x, d, J))
        if b is None or b.operator is None or b.lcont == "value":
            return self.bail(x, d, locals_, out, J)
        op = b.operator
        name = self.prims.get(op[1]) if op[0] == "G" else None
        args = [self.ref(x, d, e, f"olist[{k}]", locals_) for k, e in enumerate(b.olist)]
        lc = b.lcont
        g1 = self.funcs[lc] if isinstance(lc, int) else None
        one = g1 is not None and not g1.variadic and g1.params == 1
        if name == "__Cont_get__" and one and not b.olist and not J:
            # the escape continuation, in a Rust variable (made in a clause, it
            # would lack the CFrames of the clauses: left to the VM)
            out.append(f"let {self.xn(d)} = make_cont(m, vp);")
            return self.emit(lc, d + 1, locals_ + [(lc, 0, "v")], out, J)
        if one and name == "__is_null__" and len(b.olist) == 1 and self.rref(x, d, b.olist[0], 0, locals_) is not None:
            # a bool in a Rust variable
            out.append(f"let {self.xn(d)}: bool = matches!({self.rref(x, d, b.olist[0], 0, locals_)}, Value::Null);")
            return self.emit(lc, d + 1, locals_ + [(lc, 0, "b")], out, J)
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
                    inner = []
                    self.push_joins(J, inner)
                    out.append(f"if {bad} {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); let oe = {oe}; {' '.join(inner)} "
                               f"return finish(vp, {self.mat(d, locals_)}, gv, t, Some({lc}), Out::Call(oe, string_value({msg!r}))); }}".replace("'", '"'))
                if kind == "pred":
                    out.append(f"let {self.xn(d)}: bool = n0 {NUM_PRED[name]};")
                    return self.emit(lc, d + 1, locals_ + [(lc, 0, "b")], out, J)
                expr = {"bin": f"n0 {NUM_BIN.get(name)} n1", "div": "n0 / n1", "sqrt": "n0.sqrt()",
                        "floor": "if n0.is_finite() { n0.floor() } else { n0 }"}[kind]
                out.append(f"let {self.xn(d)}: f64 = {expr};")
                return self.emit(lc, d + 1, locals_ + [(lc, 0, "f")], out, J)
        key = b.olist[1] if name in ("__Dict_lookup_or__", "__Dict_put__") and len(b.olist) == 3 else None
        ckey = self.constant(x, d, key, 1, locals_) if key is not None else None
        if (ckey is not None and "\\" not in ckey[2] and None not in args
                and self.rref(x, d, b.olist[0], 0, locals_) is not None):
            # a String key written in place (or given to an inlined call): its
            # text, without escapes, and its hash in the Dict index (computed
            # by the Rust compiler)
            text = ckey[2][1:-1]
            err = f".map_err(|e| {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); e }})?"
            out.append(f"let a0: &Value = {self.rref(x, d, b.olist[0], 0, locals_)};")
            if name == "__Dict_put__":
                out.append(f"let a2: &Value = {self.rref(x, d, b.olist[2], 2, locals_)};")
                out.append(f'd_dict_put_str(t, {x}, a0, "{text}", {{ const H: u64 = str_hash("{text}"); H }}, a2, {{ static IC: Ic = Ic::new(); &IC }}){err};')
                out.append("let o = Out::R0;")
                return self.direct_result(x, d, locals_, lc, out, J)
            if one and self.use_kinds(lc, 0) == {"cond"}:
                # a value used only as conditions: a bool, the value tested in
                # the Dict; a missing key or a value that is not a Boolean is
                # left to the code of this Body
                out.append(f'let {self.xn(d)}: bool = match d_dict_lookup_bool_str(a0, "{text}", {{ const H: u64 = str_hash("{text}"); H }}, {{ static IC: Ic = Ic::new(); &IC }}) {{ Some(b) => b, None => {{')
                self.bail(x, d, locals_, out, J, retry=True)
                out.append("} };")
                return self.emit(lc, d + 1, locals_ + [(lc, 0, "b")], out, J)
            # the value, or onMissing applied to the key (evaluated only then)
            out.append(f'let o = match d_dict_lookup_str(t, {x}, a0, "{text}", {{ const H: u64 = str_hash("{text}"); H }}, {{ static IC: Ic = Ic::new(); &IC }}){err} {{ Some(v) => Out::R1(v), '
                       f'None => Out::Call({args[2]}, cref(t, {ckey[0]}, {ckey[1]}).clone()) }};')
            return self.direct_result(x, d, locals_, lc, out, J)
        if name in DIRECT and len(b.olist) == DIRECT[name][1] and None not in args:
            fn = DIRECT[name][0]
            names = []
            for k, e in enumerate(b.olist):
                out.append(f"let a{k}: &Value = {self.rref(x, d, e, k, locals_)};")
                names.append(f"a{k}")
            out.append(f"let o = {fn}(m, {', '.join(names)}).map_err(|e| {{ vp.sinfo = body_of(t, {x}).sinfo.clone(); e }})?;")
            return self.direct_result(x, d, locals_, lc, out, J)
        return self.emit_rest(x, d, locals_, out, b, op, name, args, lc, J)

    def direct_result(self, x, d, locals_, lc, out, J):
        """After `let o = d_...(...)`: bind the results to the LCont lc (or,
        without one, give them to the clause J or return them)."""
        sinfo_set = f"vp.sinfo = body_of(t, {x}).sinfo.clone();"
        jpush = []
        self.push_joins(J, jpush)
        jpush = " ".join(jpush)
        if lc is None:
            if not J:
                out.append(f"{sinfo_set} return finish_return(m, vp, {self.ctx['env']}.clone(), gv, t, o);")
                return
            p = J[-1]["p"]
            if p <= 2:
                vs = [f"r{k}" for k in range(p)]
                pat = ["Out::R0", "Out::R1({})", "Out::R2({}, {})"][p].format(*vs)
                tup = "(" + "".join(v + ", " for v in vs) + ")"
                out.append(f"match o {{ {pat} => break '{J[-1]['label']} {tup}, "
                           f"o => {{ {sinfo_set} {jpush} return finish(vp, {self.ctx['env']}.clone(), gv, t, None, o); }} }}")
            else:
                out.append(f"{sinfo_set} {jpush} return finish(vp, {self.ctx['env']}.clone(), gv, t, None, o);")
            return
        g = self.funcs[lc]
        fallback = f"o => {{ {sinfo_set} {jpush} return finish(vp, {self.mat(d, locals_)}, gv, t, Some({lc}), o); }}"
        if g.variadic or g.params > 2:
            out.append(f"match o {{ {fallback} }}")
            return
        vs = [self.xn(d + j) for j in range(g.params)]
        pat = ["Out::R0", "Out::R1({})", "Out::R2({}, {})"][g.params].format(*vs)
        tup = "(" + "".join(v + ", " for v in vs) + ")"
        out.append(f"let {tup} = match o {{ {pat} => {tup}, {fallback} }};")
        return self.emit(lc, d + g.params, locals_ + [(lc, j, "v") for j in range(g.params)], out, J)

    def cond(self, x, d, e, locals_):
        """Rust code for the Option<bool> of the condition e of an __if__."""
        if e[0] == "L" and e[1] < d:
            v, vk = self.local(d, locals_, e[1])
            if vk == "b":
                return f"Some({v})"
        return f"boolean_choice({self.rref(x, d, e, 0, locals_)})"

    def clauses(self, b, name):
        """The then and else clauses of an __if__ and the others: a thunk
        Function, "nop" (missing) or "value" (not written in place)."""
        def clause(i):
            c = thunk(self.funcs, b.olist[i])
            return c if c is not None else "value"
        if name == "__if_then_else__":
            return clause(1), clause(2)
        if name == "__if__":
            return clause(1), "nop"
        return "nop", clause(1)

    def margs(self, x, d, locals_, b, after):
        """The operands, the last use of a variable not in after moved."""
        mv = self.moves(d, locals_, b.olist, after)
        return [self.ref(x, d, e, f"olist[{k}]", locals_, move=mv[k]) for k, e in enumerate(b.olist)]

    def after(self, lc, d, J):
        """The chain variables read after a Body at depth d with the LCont lc
        (or none)."""
        if isinstance(lc, int):
            return self.live(lc, d + self.funcs[lc].params, J)
        return self.live(None, d, J)

    def emit_rest(self, x, d, locals_, out, b, op, name, args, lc, J):
        if op == ("Null",) and None not in args:
            args = self.margs(x, d, locals_, b, self.after(lc, d, J))
            if lc is None:
                if J:
                    return self.to_join(J, args, x, out)
                out.append(f"let v = [{', '.join(args)}]; return fused_return(m, vp, {self.ctx['env']}, {self.sinfo(x)}, v);")
                return
            g = self.funcs[lc]
            if g.variadic or g.params != len(args):
                return self.bail(x, d, locals_, out, J)
            news = []
            for j, (a, e) in enumerate(zip(args, b.olist)):
                vk = self.local(d, locals_, e[1])[1] if e[0] == "L" and e[1] < d else "v"
                c = self.constant(x, d, e, j, locals_)
                if c is not None:
                    news.append((lc, j, "c", c))   # no Rust variable
                elif vk in ("f", "b"):
                    out.append(f"let {self.xn(d + j)}: {'f64' if vk == 'f' else 'bool'} = {self.local(d, locals_, e[1])[0]};")
                    news.append((lc, j, vk))
                else:
                    out.append(f"let {self.xn(d + j)} = {a};")
                    news.append((lc, j, "v"))
            return self.emit(lc, d + g.params, locals_ + news, out, J)
        if name in IFS and len(b.olist) == IFS[name] and args[0] is not None:
            then, els = self.clauses(b, name)
            g = self.funcs[lc] if isinstance(lc, int) else None
            if "value" in (then, els) or (g is not None and g.variadic):
                return self.bail(x, d, locals_, out, J)
            if g is None:
                # no LCont: the clauses end the chain (or the clause it is in)
                out.append(f"match {self.cond(x, d, b.olist[0], locals_)} {{")
                for label, cl in (("Some(true)", then), ("Some(false)", els)):
                    out.append(f"{label} => {{")
                    if cl == "nop":
                        if J:
                            self.to_join(J, [], x, out)
                        else:
                            out.append(f"return fused_return(m, vp, {self.ctx['env']}, {self.sinfo(x)}, []);")
                    else:
                        self.emit(cl, d, locals_, out, J)
                    out.append("}")
                out.append("None => {")
                self.bail(x, d, locals_, out, J, rare=True)
                out.append("}")
                out.append("}")
                return
            # an LCont: the clauses give the values of a labeled block, and the
            # LCont goes on after it
            self.labels += 1
            label = f"j{self.labels}"
            jl = self.mark(locals_, self.after(lc, d, J))
            jn = J + ({"lc": lc, "d": d, "locals": jl, "x": x, "label": label, "p": g.params, "ctx": self.ctx},)
            vs = [self.xn(d + k) for k in range(g.params)]
            tup = "(" + "".join(v + ", " for v in vs) + ")"
            out.append(f"let {tup} = '{label}: {{")
            out.append(f"match {self.cond(x, d, b.olist[0], locals_)} {{")
            for arm, cl in (("Some(true)", then), ("Some(false)", els)):
                out.append(f"{arm} => {{")
                if cl == "nop":
                    self.to_join(jn, [], x, out)
                else:
                    self.emit(cl, d, locals_, out, jn)
                out.append("}")
            out.append("None => {")
            self.bail(x, d, locals_, out, J, rare=True)
            out.append("}")
            out.append("}")
            out.append("};")
            return self.emit(lc, d + g.params, locals_ + [(lc, k, "v") for k in range(g.params)], out, J)
        callable_op = op[0] == "L" or (op[0] == "G" and op[1] not in self.prims)
        f_ref = self.ref(x, d, op, "operator", locals_) if callable_op else None
        if callable_op and isinstance(lc, int) and None not in args and f_ref is not None:
            # a call with an LCont: a Rust call, whose values the chain binds
            # (or the code of the Function called, written in place)
            if op[0] == "G" and op[1] in self.gfun and self.inline(x, d, locals_, b, f_ref, lc, J, self.gfun[op[1]], out):
                return
            g = self.funcs[lc]
            after = self.after(lc, d, J)
            args = self.margs(x, d, locals_, b, after)
            lm = self.mark(locals_, after)   # (without the variables moved)
            mat = self.mat(d, lm)
            jpush = []
            self.push_joins(J, jpush)
            jpush = " ".join(jpush)
            out.append(f"let f = {f_ref};")
            # the arguments in an array (no heap), the values bound from the
            # OList (which keeps its buffer)
            out.append(f"let v = [{', '.join(args)}];")
            out.append(f"let o = fused_call(m, vp, f, v, gv, t, {self.node(lc, d, lm, x, J)}, {self.sinfo(x)})?;")
            if g.variadic:
                out.append(f"match o {{ Called::Values => {{ {jpush} return fused_pass(vp, gv, t, {lc}, {mat}); }}, Called::Next(n) => return Ok(n) }}")
                return
            out.append(f"match o {{ Called::Values if vp.olist.len() == {g.params} => {{}}, "
                       f"Called::Values => {{ {jpush} return fused_pass(vp, gv, t, {lc}, {mat}); }}, Called::Next(n) => return Ok(n) }}")
            if g.params:
                out.append("let mut v = vp.olist.drain(..);")
                for j in range(g.params):
                    out.append(f"let {self.xn(d + j)} = v.next().unwrap();")
                out.append("drop(v);")
            return self.emit(lc, d + g.params, locals_ + [(lc, j, "v") for j in range(g.params)], out, J)
        if callable_op and lc is None and None not in args:
            args = self.margs(x, d, locals_, b, self.after(None, d, J))
            out.append(f"let f = {f_ref};")
            if not J:
                lp = self.loop
                if lp is not None and len(args) == lp["p"]:
                    # the Closure running (same Function and LVEnv): its new
                    # arguments in the parameters' variables, and again
                    lp["used"] = True
                    news = "".join(f"let v{j} = {a}; " for j, a in enumerate(args))
                    sets = "".join(f"{self.xn(j)} = v{j}; " for j in range(len(args)))
                    out.append(f"if self_call(m, vp, &f, {lp['fid']}, {lp['p']}, &{self.ctx['env']}, gv) {{ {news}{sets}full = None; continue 'l0; }}")
                out.append(f"let v = [{', '.join(args)}]; return fused_tail_call(vp, {self.ctx['env']}, {self.sinfo(x)}, f, v);")
                return
            # in a clause: the call's LCont is the clause's. A continuation
            # (return) drops the LCont: left to apply as it is.
            j = J[-1]
            outer = []
            self.push_joins(J[:-1], outer)
            outer = " ".join(outer)
            out.append(f"if matches!(f, Value::Cont(_)) {{ let v = [{', '.join(args)}]; return fused_tail_call(vp, {self.ctx['env']}, {self.sinfo(x)}, f, v); }}")
            out.append(f"let v = [{', '.join(args)}];")
            out.append(f"let o = fused_call(m, vp, f, v, gv, t, {self.node(None, d, self.mark(locals_, self.after(None, d, J)), x, J)}, &body_of(t, {j['x']}).sinfo)?;")
            vs = [f"r{k}" for k in range(j["p"])]
            tup = "(" + "".join(v + ", " for v in vs) + ")"
            binds = " ".join(f"let r{k} = v.next().unwrap();" for k in range(j["p"]))
            out.append(f"match o {{ Called::Values if vp.olist.len() == {j['p']} => {{ let mut v = vp.olist.drain(..); {binds} drop(v); break '{j['label']} {tup}; }}, "
                       f"Called::Values => {{ {outer} return fused_pass(vp, gv, t, {j['lc']}, {self.mat(j['d'], j['locals'], j['ctx'])}); }}, Called::Next(n) => return Ok(n) }}")
            return
        return self.bail(x, d, locals_, out, J)

    # ── Inlining ──
    #
    # A call with an LCont of a global variable whose value is a small
    # top-level Function (INLINE_SIZE Bodies at most, without __Cont_get__,
    # not itself or one it is written in) is written as the code of that
    # Function in a labeled block, as a clause with an LCont is: what the
    # Function returns is the value of the block, and the LCont goes on after
    # it. The code runs in the LVEnv of the Closure (checked to be one of
    # that Function, inline_env, as the global variable could hold another
    # value; if not, the call is made as above), its parameters in Rust
    # variables numbered after those of the caller, and a String written in
    # place given to it stays a constant (so that a Dict lookup with it as
    # the key is one with a String key written in place). Where it hands the
    # VProc back, the CFrame of the caller's LCont is put on the CChain, as
    # for a clause. A Function whose code would hand the VProc back on its
    # usual path (a bail that is not for a rare case) is not inlined.

    def size(self, fid):
        b = self.funcs[fid].body
        if b is None:
            return 1
        n = 1 + (self.size(b.lcont) if isinstance(b.lcont, int) else 0)
        return n + sum(self.size(e[1]) for e in b.olist if e[0] == "F")

    def has_cont_get(self, fid):
        b = self.funcs[fid].body
        if b is None:
            return False
        if b.operator is not None and b.operator[0] == "G" and self.prims.get(b.operator[1]) == "__Cont_get__":
            return True
        fs = ([b.lcont] if isinstance(b.lcont, int) else []) + [e[1] for e in b.olist if e[0] == "F"]
        return any(self.has_cont_get(f) for f in fs)

    def inline(self, x, d, locals_, b, f_ref, lc, J, gf, out):
        """Write the call (Body of Function x, with the LCont lc) of the
        global Function gf in place, if it can be; whether it was."""
        g, L = self.funcs[gf], self.funcs[lc]
        if (g.body is None or g.variadic or g.params != len(b.olist) or L.variadic
                or len(self.inlining) >= INLINE_DEPTH or gf in self.inlining or gf == self.loop["fid"]
                or self.size(gf) > INLINE_SIZE or self.has_cont_get(gf)):
            return False
        after = self.after(lc, d, J)
        lm = self.mark(locals_, after)
        mv = self.moves(d, locals_, b.olist, after)
        caller = self.ctx
        callee = {"env": None, "off": caller["off"] + d, "hp": 0}
        # its parameters: constants, or Rust variables (moved at their last use)
        binds, entries = [], []
        for k, e in enumerate(b.olist):
            c = self.constant(x, d, e, k, locals_)
            vk = self.local(d, locals_, e[1])[1] if e[0] == "L" and e[1] < d else "v"
            name = f"x{callee['off'] + k}"
            if c is not None:
                entries.append((gf, k, "c", c))
            elif vk in ("f", "b"):
                binds.append(f"let {name}: {'f64' if vk == 'f' else 'bool'} = {self.local(d, locals_, e[1])[0]};")
                entries.append((gf, k, vk))
            else:
                a = self.ref(x, d, e, f"olist[{k}]", locals_, move=mv[k])
                if a is None:
                    return False
                binds.append(f"let {name} = {a};")
                entries.append((gf, k, "v"))
        saved = (set(self.bails), set(self.retries), self.nbail, self.labels, self.envs)
        self.envs += 1
        callee["env"] = f"e{self.envs}"
        self.labels += 1
        label = f"j{self.labels}"
        jn = J + ({"lc": lc, "d": d, "locals": lm, "x": x, "label": label, "p": L.params, "ctx": caller},)
        inner = []
        self.ctx = callee
        self.inlining.append(gf)
        try:
            self.emit(gf, g.params, entries, inner, jn)
        finally:
            self.ctx = caller
            self.inlining.pop()
        if self.nbail != saved[2]:
            self.bails, self.retries, self.nbail, self.labels, self.envs = saved
            return False
        vs = [self.xn(d + k) for k in range(L.params)]
        tup = "(" + "".join(v + ", " for v in vs) + ")"
        out.append(f"let f = {f_ref};")
        out.append(f"let {tup} = '{label}: {{")
        out.append(f"if let Some({callee['env']}) = inline_env(vp, &f, {gf}, {g.params}, gv) {{")
        out.extend(binds)
        out.extend(inner)
        out.append("}")
        # not that Function: the call, as above
        args = self.margs(x, d, locals_, b, after)
        jpush = []
        self.push_joins(J, jpush)
        rs = [f"r{k}" for k in range(L.params)]
        rtup = "(" + "".join(v + ", " for v in rs) + ")"
        binds = " ".join(f"let r{k} = v.next().unwrap();" for k in range(L.params))
        out.append(f"let v = [{', '.join(args)}];")
        out.append(f"let o = fused_call(m, vp, f, v, gv, t, {self.node(lc, d, lm, x, J)}, {self.sinfo(x)})?;")
        out.append(f"match o {{ Called::Values if vp.olist.len() == {L.params} => {{ let mut v = vp.olist.drain(..); {binds} drop(v); break '{label} {rtup}; }}, "
                   f"Called::Values => {{ {' '.join(jpush)} return fused_pass(vp, gv, t, {lc}, {self.mat(d, lm)}); }}, Called::Next(n) => return Ok(n) }}")
        out.append("};")
        self.emit(lc, d + L.params, locals_ + [(lc, k, "v") for k in range(L.params)], out, J)
        return True


def translate(index, path, prim_names, out):
    data = Path(path).read_bytes()
    module = parse(data.decode("utf-8"))
    if tag(module) != "Module":
        raise ValueError("no Module")
    funcs, prims, tops, gfun = load(module, prim_names)
    used = []

    def prim_index(name):
        if name not in used:
            used.append(name)
        return used.index(name)
    calls = [transpiled_call(funcs, prims, f, prim_index) for f in funcs]
    directs = {}
    Fuser(funcs, prims, tops, gfun).apply(calls, directs)
    p = f"m{index}"
    out.append(f"// {path}: {len(funcs)} Functions, {sum(n is not None for n in calls)} with transpiled code, "
               f"{len(directs)} with a direct entry")
    for f, call in zip(funcs, calls):
        if call is not None:
            call = re.sub(r"\bDIRECT_(\d+)\b", lambda mo: f"{p}_d{mo.group(1)}", call)
            out.append(f"fn {p}_f{f.id}(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> NResult {{ {call} }}")
        if f.id in directs:
            out.append(f"fn {p}_d{f.id}(m: &mut Machine, vp: &mut VProc, env: LVEnv, a: &mut [Value], full: Option<LVEnv>, "
                       f"gv: &Rc<GVEnv>, t: &Table) -> NResult {{ {directs[f.id]} }}")
    entries = ", ".join(f"Some({p}_f{f.id} as TranspiledFn)" if c is not None else "None" for f, c in zip(funcs, calls))
    out.append(f"static {p.upper()}_FNS: [Option<TranspiledFn>; {len(funcs)}] = [{entries}];")
    dentries = ", ".join(f"Some({p}_d{f.id} as DirectFn)" if f.id in directs else "None" for f in funcs)
    out.append(f"static {p.upper()}_DFNS: [Option<DirectFn>; {len(funcs)}] = [{dentries}];")
    names = ", ".join(f'"{n}"' for n in used)
    return (f"Module {{ hash: 0x{fnv1a(data):016x}, nfuncs: {len(funcs)}, "
            f"prims: &[{names}], fns: &{p.upper()}_FNS, dfns: &{p.upper()}_DFNS }}")


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
