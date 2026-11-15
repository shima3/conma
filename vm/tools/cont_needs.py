#!/usr/bin/env python3
"""Classify the global functions of a resolved ConMa module (a .module.se, such as the modules of ConSh in
bin/cache-dir) by the weakest form of control they need, taking the functions they call into account. It
tells how much of a program a compiler could run on a native stack with ordinary calls and returns.

    python3 vm/tools/cont_needs.py [--list TIER] MODULE...

Tiers, from the weakest (a function is in the strongest tier it or a function it calls needs):
  D  direct: ordinary calls and returns only
  E  escapes: __Cont_get__ whose continuation is only called, or passed to a function or a primitive (a
     non-local exit, like an exception; a continuation passed on is assumed to be called while the
     function runs)
  H  calls a closure it got as a value (a parameter or a result): direct only if the closures passed are
  I  inspects the stack (Sjs_raise_pos reads the SInfo of the CFrames): a shadow stack of SInfo would do
  W  may suspend its VProc (__VProc_wait__, __Ref_wait_change__, __LazyList_claim__, __OS_wait*__)
  K  needs real continuations: resumes a stream (__Seq_get__), reifies the CChain, or stores a
     continuation in a data structure
__CChain_pop_LCont__, __noop__ and __print__ of include/util.se take their own LCont only to return to it,
and count as direct. The functions of include/core (the primitives of extended ConMa written in core ConMa,
such as __Cont_get__ and __if__ in the module of the reference version) count as primitives: their uses are
classified where they are called. The analysis is static and counts functions, not the time spent in them.
"""
import argparse
import collections
import os
import re
import sys

_TOKEN = re.compile(r'\s*(?:(\()|(\))|"((?:[^"\\]|\\.)*)"|([^\s()"]+))')


def parse(text):
    """The S-expression of a module, as nested lists of strings."""
    stack = [[]]
    for m in _TOKEN.finditer(text):
        open_, close, string, atom = m.groups()
        if open_:
            stack.append([])
        elif close:
            x = stack.pop()
            stack[-1].append(x)
        elif string is not None:
            stack[-1].append(string)
        elif atom is not None:
            stack[-1].append(atom)
    return stack[0][0]


ORDER = 'KWIHED'
SPECIAL = {'__CChain_pop_LCont__': set(), '__noop__': set(), '__print__': set(),
           'Sjs_raise_pos': {'I'}, 'Sjs_raise_pos_scan': {'I'}, '__Seq_get__': {'K'}}
WPRIM = {'__VProc_wait__', '__Ref_wait_change__', '__LazyList_claim__', '__OS_wait__', '__OS_wait_readable__',
         '__OS_wait_writable__'}
KPRIM = {'__CChain_get__', '__CChain_set__', '__CChain_pop_CFrame__', '__CFrame_get_Closure__', '__CFrame_get_SInfo__'}
STORE = {'__Dict_put__', '__PDict_put__', '__Ref_new__', '__Ref_set__', '__LList_cons__', '__LList_append__',
         '__MSeq_append__', '__VProc_spawn__', '__VProc_start__'}

def is_var(x): return isinstance(x, list) and x and x[0] == 'Variable'
def gref(x): return is_var(x) and len(x) > 3 and isinstance(x[3], list) and x[3][0] == 'G'
def part(x, tag): return next((y for y in x if isinstance(y, list) and y and y[0] == tag), None)

def first_file(x):
    """The file of the first SInfo in x (\"\" if none)."""
    if isinstance(x, list) and x:
        if x[0] == 'SInfo':
            return x[2]
        for y in x[1:]:
            f = first_file(y)
            if f:
                return f
    return ""


def analyze(path):
    m = parse(open(path).read())
    gs = {g[2]: g for g in m[1:] if isinstance(g, list) and g[0] == 'G'}
    prims = {n for n, g in gs.items() if g[3] == 'Undefined'}
    funcs = {n: g[3] for n, g in gs.items() if n not in prims}
    own, calls, files = {}, {}, {}
    for name, fn in funcs.items():
        flags, callees, contvars = set(), set(), set()
        def collect(x):
            if isinstance(x, list) and x:
                if x[0] == 'Body':
                    op, lc = part(x, 'Operator'), part(x, 'LCont')
                    if op is not None and len(op) > 2 and gref(op[2]) and op[2][2] == '__Cont_get__' \
                            and lc is not None and len(lc) > 2:
                        contvars.update(v[2] for v in lc[2][2][2:] if is_var(v))
                for y in x[1:]: collect(y)
        collect(fn)
        def walk(x):
            if not isinstance(x, list) or not x: return
            if x[0] == 'SInfo' and name not in files: files[name] = x[2]
            if x[0] == 'Body':
                op, ol = part(x, 'Operator'), part(x, 'OList')
                o = op[2] if op is not None and len(op) > 2 else None
                if o is not None and gref(o) and o[2] == '__Cont_get__':
                    flags.add('E')
                elif o is not None and is_var(o) and not gref(o) and o[2] not in contvars:
                    flags.add('H')
                if ol is not None:
                    for a in ol[2:]:
                        if is_var(a) and not gref(a) and a[2] in contvars:
                            # a continuation passed as a value: stored, or called by the receiver
                            if o is not None and gref(o) and o[2] in STORE: flags.add('K')
            if gref(x):
                n = x[2]
                if n in KPRIM and name not in SPECIAL: flags.add('K')
                elif n in WPRIM: flags.add('W')
                elif n in funcs: callees.add(n)
            for y in x[1:]: walk(y)
        if "/include/core/" in first_file(fn):
            # a primitive of extended ConMa written in core ConMa (the reference version): like a primitive
            own[name], calls[name] = set(), set()
            continue
        if name in SPECIAL:
            flags = set(SPECIAL[name]); walk(fn); flags = set(SPECIAL[name]) | {f for f in flags if f == 'W'}
        else:
            walk(fn)
        own[name], calls[name] = flags, callees
    tot = {n: set(f) for n, f in own.items()}
    changed = True
    while changed:
        changed = False
        for n in funcs:
            for c in calls[n]:
                add = tot[c] - tot[n]
                if add: tot[n] |= add; changed = True
    return funcs, own, tot, files, calls

def tier(f):
    for t in ORDER:
        if t in f: return t
    return 'D'


def main():
    ap = argparse.ArgumentParser(description="Classify the functions of ConMa modules by the control they need.")
    ap.add_argument("--list", metavar="TIER", help="also list the functions of a tier (D, E, H, I, W or K)")
    ap.add_argument("modules", nargs="+")
    a = ap.parse_args()
    for path in a.modules:
        funcs, own, tot, files, calls = analyze(path)
        print(f"== {os.path.basename(path)}: {len(funcs)} functions (columns D E H I W K)")
        for label, fl in (("own code", own), ("with callees", tot)):
            c = collections.Counter(tier(f) for f in fl.values())
            print(f"  {label:13s}" + " ".join(f"{c[t]:5d}" for t in "DEHIWK"))
        byfile = collections.defaultdict(collections.Counter)
        for n, f in tot.items():
            byfile[files.get(n, "?").split("conma/")[-1]][tier(f)] += 1
        for fpath in sorted(byfile, key=lambda p: -sum(byfile[p].values())):
            c = byfile[fpath]
            print(f"    {fpath:36s}" + " ".join(f"{c[t]:5d}" for t in "DEHIWK"))
        hub = collections.Counter()
        for n, f in tot.items():
            if "K" in f:
                for c in calls[n]:
                    if "K" in tot[c]:
                        hub[c] += 1
        print("  callees that pass K on most often:", ", ".join(f"{n} {k}" for n, k in hub.most_common(10)))
        if a.list:
            names = sorted(n for n, f in tot.items() if tier(f) == a.list)
            print(f"  tier {a.list}:", " ".join(names))


if __name__ == "__main__":
    main()
