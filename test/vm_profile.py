#!/usr/bin/env python3
"""Sum the step counts of the Rust VM (CONMA_PROFILE=<file>) by function.

    CONMA_PROFILE=/tmp/prof.tsv vm/target/release/conma ...
    python3 test/vm_profile.py [--top N] /tmp/prof.tsv

Each Body is counted under the top-level (define NAME ...) that contains its
line. A call is the start of the first Body of a function; --callers NAME shows
the lines that call NAME. Paths in the module start with "conma/" (bin/consh and test/resolver.sh
write them so); they are read from the repository.
"""

import argparse
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFINE = re.compile(r'^\(define\s+([^\s()]+)')


def source_path(name):
    if name.startswith("conma/"):
        return os.path.join(ROOT, name[len("conma/"):])
    return name


_defines = {}


def defines(name):
    """[(line, define name)] of a source file, in order."""
    if name not in _defines:
        found = []
        try:
            with open(source_path(name), encoding="utf-8") as f:
                for i, text in enumerate(f, 1):
                    m = DEFINE.match(text)
                    if m:
                        found.append((i, m.group(1)))
        except OSError:
            pass
        _defines[name] = found
    return _defines[name]


def function_of(name, line):
    best = "(top)"
    for i, d in defines(name):
        if i > line:
            break
        best = d
    return best


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--top", type=int, default=30)
    ap.add_argument("--callers", metavar="NAME",
                    help="show who calls the function NAME, by calling function")
    ap.add_argument("--thunks", action="store_true",
                    help="show the lines that pass a condition written as a function (not a Boolean) to __if__, __unless__ or __if_then_else__")
    ap.add_argument("profile")
    args = ap.parse_args()
    funcs = defaultdict(int)
    files = defaultdict(int)
    prims = defaultdict(int)
    calls = defaultdict(int)
    callers = defaultdict(int)
    thunks = defaultdict(int)
    total = 0
    records = []
    with open(args.profile, encoding="utf-8") as f:
        for text in f:
            records.append(text.rstrip("\n").split("\t"))
    # The Body that a function starts with: the first of the function in the
    # source. Entering the other Bodies of a function (its LConts and the
    # Closures it passes, such as the clauses of __if__) is not a call.
    first = {}
    for parts in records:
        if parts[1] in ("body", "call") and len(parts) >= 5:
            name, pos = parts[2], (int(parts[3]), int(parts[4]))
            key = (name, function_of(name, pos[0]))
            if key not in first or pos < first[key]:
                first[key] = pos

    def is_start(name, line, col):
        return first.get((name, function_of(name, line))) == (line, col)

    for parts in records:
        n = int(parts[0])
        if parts[1] == "prim":
            prims[parts[2]] += n
            continue
        if parts[1] == "edge":
            callee = function_of(parts[5], int(parts[6]))
            if callee == args.callers and is_start(parts[5], int(parts[6]), int(parts[7])):
                if parts[2] == "-":
                    caller = "(no SInfo)"
                else:
                    caller = f"{function_of(parts[2], int(parts[3]))} ({os.path.basename(parts[2])}:{parts[3]})"
                callers[caller] += n
            continue
        if parts[1] == "thunk":
            thunks[f"{os.path.basename(parts[2])}:{parts[3]} {function_of(parts[2], int(parts[3]))}"] += n
            continue
        if parts[1] == "call":
            if is_start(parts[2], int(parts[3]), int(parts[4])):
                short = os.path.basename(parts[2])
                calls[f"{function_of(parts[2], int(parts[3]))} ({short})"] += n
            continue
        total += n
        if len(parts) < 4:
            funcs["(no SInfo)"] += n
            continue
        name, line = parts[2], int(parts[3])
        short = os.path.basename(name)
        files[short] += n
        funcs[f"{function_of(name, line)} ({short})"] += n

    def show(title, table, denom):
        print(f"== {title}")
        for k, n in sorted(table.items(), key=lambda kv: -kv[1])[:args.top]:
            print(f"{n:>12,} {100 * n / denom:5.1f}%  {k}")
        print()

    if args.thunks:
        print("== conditions written as functions (file:line function)")
        for k, n in sorted(thunks.items(), key=lambda kv: -kv[1])[:args.top]:
            print(f"{n:>12,}  {k}")
        return

    if args.callers:
        print(f"== callers of {args.callers} (calling function and line)")
        for k, n in sorted(callers.items(), key=lambda kv: -kv[1])[:args.top]:
            print(f"{n:>12,}  {k}")
        return

    print(f"steps: {total:,}\n")
    show("files", files, total)
    print("== functions: steps, share, calls, steps per call")
    for k, n in sorted(funcs.items(), key=lambda kv: -kv[1])[:args.top]:
        c = calls.get(k, 0)
        per = f"{n / c:7.1f}" if c else "      -"
        print(f"{n:>12,} {100 * n / total:5.1f}% {c:>10,} {per}  {k}")
    print()
    show("primitives (steps that apply them)", prims, total)


if __name__ == "__main__":
    main()
