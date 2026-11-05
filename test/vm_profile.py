#!/usr/bin/env python3
"""Sum the step counts of the Rust VM (CONMA_PROFILE=<file>) by function.

    CONMA_PROFILE=/tmp/prof.tsv vm/target/release/conma ...
    python3 test/vm_profile.py [--top N] /tmp/prof.tsv

Each Body is counted under the top-level (define NAME ...) that contains its
line. Paths in the module start with "conma/" (bin/consh and test/resolver.sh
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
    ap.add_argument("profile")
    args = ap.parse_args()
    funcs = defaultdict(int)
    files = defaultdict(int)
    prims = defaultdict(int)
    calls = defaultdict(int)
    total = 0
    with open(args.profile, encoding="utf-8") as f:
        for text in f:
            parts = text.rstrip("\n").split("\t")
            n = int(parts[0])
            if parts[1] == "prim":
                prims[parts[2]] += n
                continue
            if parts[1] == "call":
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
