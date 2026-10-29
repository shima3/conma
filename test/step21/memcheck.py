#!/usr/bin/env python3
"""memcheck.py — Check that the lines already read from standard input are collected.

Usage: memcheck.py <module_file> <mode> < input

Runs the program like `vproc_run.py --main <module_file> <mode>`. Every
INTERVAL steps it runs the garbage collector and counts the LList cells of
the interpreter that hold a line of the input (a String that starts with
"line "), and at the end prints the largest count. If the lines already
read are collected, it stays small however long the input is.
"""

import gc, sys, pathlib, importlib.util

INTERVAL = 2000

spec = importlib.util.spec_from_file_location(
    "vproc_run", pathlib.Path(__file__).parent.parent / "vproc_run.py")
vproc_run = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vproc_run)
vs = vproc_run.vs

def count_line_cells():
    gc.collect()
    return sum(1 for o in gc.get_objects()
               if type(o) is vs.LList and isinstance(o.head, str) and o.head.startswith('"line '))

def main():
    module_file, mode = sys.argv[1], sys.argv[2]
    module_sexp = vs.parse_sexp(open(module_file).read())
    gid   = vs.build_name_index(module_sexp)["__start__"]
    gvenv = vs.load_module(module_sexp)
    olist = [vs.SeqClosure([vs.string_value(mode)], None, [])]
    vp    = vs.make_vproc(gvenv.get(gid), olist, module_file, gvenv)
    scheduler = vs.SCHEDULER
    scheduler.add(vp)
    largest = 0
    def on_step(v):
        nonlocal largest
        if scheduler.steps % INTERVAL == 0:
            largest = max(largest, count_line_cells())
    scheduler.run(on_step=on_step)
    print(f"largest number of live line cells: {largest}")

if __name__ == "__main__":
    main()
