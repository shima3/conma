#!/usr/bin/env python3
"""Sum the counts of the transpiled VM (CONMA_TRANSPILED_COUNTS=<file>): where
the transpiled code hands the VProc back, by Body and by function.

    CONMA_TRANSPILED_COUNTS=/tmp/counts.tsv vm/target-transpiled/release/conma ...
    python3 test/transpiled_counts.py [--top N] /tmp/counts.tsv

Kinds (vm/src/transpiled.rs, "Counts"):
  bail   a fused chain entered the code of the Body above (it could not go on)
  enter  the code of a Body was entered (after a bail, the code of a Body
         going on with the next one, a Closure without a direct entry)
  deopt  the Body was loaded for the VM (its code could not run it)
  run    the VM entered transpiled code at the Body
  leave  a step was left to the VM, by what it applied
Each Body is shown with the top-level (define NAME ...) that contains it and
the text of its line.
"""

import argparse
import os
import sys
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vm_profile import function_of, source_path  # noqa: E402

KINDS = ["bail", "enter", "deopt", "run"]

_lines = {}


def line_text(name, line):
    if name not in _lines:
        try:
            with open(source_path(name), encoding="utf-8") as f:
                _lines[name] = f.read().split("\n")
        except OSError:
            _lines[name] = []
    text = _lines[name]
    return text[line - 1].strip() if 0 < line <= len(text) else ""


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--top", type=int, default=15)
    ap.add_argument("counts")
    args = ap.parse_args()
    bodies = defaultdict(lambda: defaultdict(int))   # kind -> (file, line, col) -> n
    funcs = defaultdict(lambda: defaultdict(int))    # kind -> (function, file) -> n
    leaves = defaultdict(int)
    with open(args.counts, encoding="utf-8") as f:
        for text in f:
            r = text.rstrip("\n").split("\t")
            n, kind = int(r[0]), r[1]
            if kind == "leave":
                leaves["\t".join(r[2:])] += n
            elif len(r) >= 5:
                name, line, col = r[2], int(r[3]), int(r[4])
                bodies[kind][(name, line, col)] += n
                funcs[kind][(function_of(name, line), os.path.basename(name))] += n
            else:
                bodies[kind][("-", 0, 0)] += n
    for kind in KINDS:
        total = sum(bodies[kind].values())
        if not total:
            continue
        print(f"== {kind}: {total:,}")
        print("  by function:")
        for (fn, name), n in sorted(funcs[kind].items(), key=lambda x: -x[1])[:args.top]:
            print(f"  {n:>10,} {n / total:6.1%}  {fn} ({name})")
        print("  by Body:")
        for (name, line, col), n in sorted(bodies[kind].items(), key=lambda x: -x[1])[:args.top]:
            where = f"{os.path.basename(name)}:{line}:{col}"
            print(f"  {n:>10,} {n / total:6.1%}  {where:<26} {line_text(name, line)[:70]}")
        print()
    total = sum(leaves.values())
    if total:
        print(f"== leave: {total:,}")
        for what, n in sorted(leaves.items(), key=lambda x: -x[1])[:args.top]:
            print(f"  {n:>10,} {n / total:6.1%}  {what}")


if __name__ == "__main__":
    main()
