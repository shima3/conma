#!/usr/bin/env python3
"""translate.py — make native code (Rust) for ConMa modules.

Usage:
    python3 vm/native/translate.py OUT.rs MODULE.module.se ...

Each MODULE is the output of bin/resolver (test/resolver.sh). OUT.rs is
included by vm/src/native.rs when the VM is built with CONMA_NATIVE=OUT.rs
(vm/native/build.sh). A module's native code is used only for a module file
with the same text (its FNV-1a hash).

The Functions of a module are numbered in the order in which load_module in
vm/src/module.rs compiles them. For each Function whose Body can start
running without the VM, a native function is made that calls a helper of
vm/src/native.rs:

* prim_call: the Operator is a primitive of CHAIN (one that only works on its
  operands and returns its results or applies onError), with an LCont written
  in place or none.
* null_pass: the Operator is __NULL__ with an LCont written in place.
* if_call: __if_then_else__, __if__ or __unless__. A condition `(,() p a ...)`
  with a primitive p of PURE is computed in place; clauses `(,() ...)` are
  run in place.

The helpers go on with the native function of the next Body, so a run of such
Bodies takes one step of the VM.
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
    "__Number_subtract__", "__Number_multiply__", "__Number_divide__", "__Number_floor__",
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
# run of native code would not wake them, as it does in the VM.

# Primitives of CHAIN without side effects that return one value: a condition
# thunk calling one of them is computed in place (and again by the VM if the
# result is not a Boolean).
PURE = {
    "__is_null__", "__type_name__", "__Number_is_zero__", "__Number_is_positive__",
    "__Number_is_negative__", "__String_equal__", "__String_length__",
    "__LList_length__", "__Ref_get__",
}

IFS = {"__if_then_else__": 3, "__if__": 2, "__unless__": 2}


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
            return ("C",)
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
            comp.function(value)
    return comp.funcs, prims


# ── Code ──

def thunk(funcs, expr):
    """The Function of `(,() ...)`, or None."""
    if expr[0] != "F":
        return None
    f = funcs[expr[1]]
    if f.params == 0 and not f.variadic and f.body is not None:
        return f.id
    return None


def native_call(funcs, prims, f, prim_index):
    """The helper call of the native function of f, or None."""
    b = f.body
    if b is None or b.lcont == "value" or b.operator is None:
        return None
    lc = "None" if b.lcont is None else f"Some({b.lcont})"
    op = b.operator
    if op == ("Null",):
        return None if b.lcont is None else f"null_pass(m, vp, env, gv, t, {f.id}, {b.lcont})"
    if op[0] != "G" or op[1] not in prims:
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
    if name in CHAIN:
        return f"prim_call(m, vp, env, gv, t, {f.id}, {prim_index(name)}, {lc})"
    return None


def translate(index, path, prim_names, out):
    data = Path(path).read_bytes()
    module = parse(data.decode("utf-8"))
    if tag(module) != "Module":
        raise ValueError("no Module")
    funcs, prims = load(module, prim_names)
    used = []

    def prim_index(name):
        if name not in used:
            used.append(name)
        return used.index(name)
    natives = [native_call(funcs, prims, f, prim_index) for f in funcs]
    p = f"m{index}"
    out.append(f"// {path}: {len(funcs)} Functions, {sum(n is not None for n in natives)} with native code")
    for f, call in zip(funcs, natives):
        if call is not None:
            out.append(f"fn {p}_f{f.id}(m: &mut Machine, vp: &mut VProc, env: LVEnv, gv: &Rc<GVEnv>, t: &Table) -> PResult {{ {call} }}")
    entries = ", ".join(f"Some({p}_f{f.id} as NativeFn)" if c is not None else "None" for f, c in zip(funcs, natives))
    out.append(f"static {p.upper()}_NATIVES: [Option<NativeFn>; {len(funcs)}] = [{entries}];")
    names = ", ".join(f'"{n}"' for n in used)
    return (f"Module {{ hash: 0x{fnv1a(data):016x}, nfuncs: {len(funcs)}, "
            f"prims: &[{names}], natives: &{p.upper()}_NATIVES }}")


def prim_names_of_vm():
    src = (Path(__file__).resolve().parent.parent / "src" / "prim" / "mod.rs").read_text()
    return set(re.findall(r'^\s*"(__\w+__)"\s*=>', src, re.M))


def main():
    if len(sys.argv) < 3:
        print(__doc__.strip().splitlines()[2].strip(), file=sys.stderr)
        sys.exit(2)
    prim_names = prim_names_of_vm()
    out = ["// Made by vm/native/translate.py. Do not edit."]
    modules = []
    for path in sys.argv[2:]:
        try:
            modules.append(translate(len(modules), path, prim_names, out))
        except (IndexError, ValueError, UnicodeDecodeError) as e:
            print(f"translate.py: {path}: not a module ({e!r}); skipped", file=sys.stderr)
    out.append("pub static MODULES: &[Module] = &[" + ", ".join(modules) + "];")
    Path(sys.argv[1]).write_text("\n".join(out) + "\n")


if __name__ == "__main__":
    main()
