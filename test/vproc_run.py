#!/usr/bin/env python3
"""vproc_run.py — Run a ConMa VProc to completion.

Usage:
    vproc_run.py [options] <module_file> <func_name>
    vproc_run.py [options] --main <module_file> [<arg> ...]

The first form applies the function <func_name> to no arguments.
The second form starts the program the normal way: it applies __start__
(include/util.se) to a Seq of the <arg> Strings; __start__ makes err, in and
out from the interpreter's standard streams and calls __main__.

Options:
    -m, --main      Start the program with __start__ and pass the <arg>s.
    -t, --trace     Print the VProc state to stderr before each step.
    -h, --help      Show this message and exit.

Exit codes:
    0   Every VProc has finished. A VProc that stopped with a runtime error
        (VProcError) does not change the exit code.
    1   __OS_exit_error__ was executed, or the module could not be loaded.
    2   Deadlock: no VProc is ready but some are blocked, or a wrong
        command line (reported by argparse).
"""

import sys, pathlib, importlib.util, argparse

# ── Import vproc_step ─────────────────────────────────────────────────────────

def _load_vproc_step():
    for candidate in [
        pathlib.Path(__file__).parent / "vproc_step.py",
        pathlib.Path("vproc_step.py"),
        pathlib.Path(__file__).parent / "vproc_step8.py",
        pathlib.Path("vproc_step8.py"),
    ]:
        if candidate.exists():
            spec = importlib.util.spec_from_file_location("vproc_step", candidate)
            mod  = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            return mod
    raise ImportError("vproc_step.py not found")

try:
    vs = _load_vproc_step()
except ImportError as e:
    print(f"Error: {e}", file=sys.stderr)
    sys.exit(1)

parse_sexp    = vs.parse_sexp
sexp_to_str   = vs.sexp_to_str
load_module   = vs.load_module
value_to_sexp = vs.value_to_sexp
vproc_to_sexp = vs.vproc_to_sexp
unquote       = vs.unquote
quoted        = vs.quoted
is_null_value = vs.is_null_value
VProcError    = vs.VProcError

build_name_index = vs.build_name_index

# ── Trace helper ──────────────────────────────────────────────────────────────

def print_trace(vp, step_n):
    label = f"[step {step_n}]" if step_n > 0 else "[initial]"
    if vp.serial != 1:
        label = label[:-1] + f" vproc {vp.serial}]"
    print(f"{label} {sexp_to_str(vproc_to_sexp(vp))}", file=sys.stderr)

# ── Runner ────────────────────────────────────────────────────────────────────

def run(module_file, func_name, trace=False, program_args=None):
    try:
        module_text = open(module_file).read()
    except OSError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    module_sexp = parse_sexp(module_text)
    name_index  = build_name_index(module_sexp)

    if func_name not in name_index:
        available = ", ".join(sorted(name_index))
        print(
            f"Error: function {func_name!r} not found in module.\n"
            f"Available: {available}",
            file=sys.stderr,
        )
        sys.exit(1)

    gid   = name_index[func_name]
    gvenv = load_module(module_sexp)

    closure = gvenv.get(gid)
    if closure is None:
        print(f"Error: GID {gid} ({func_name!r}) is undefined.", file=sys.stderr)
        sys.exit(1)

    olist = []
    if program_args is not None:
        olist = [vs.SeqClosure([vs.string_value(a) for a in program_args], None, [])]
    vp        = vs.make_vproc(closure, olist, module_file, gvenv)
    scheduler = vs.SCHEDULER
    scheduler.add(vp)

    if trace:
        print_trace(vp, 0)

    scheduler.run(on_step=(lambda v: print_trace(v, scheduler.steps)) if trace else None)

    if trace:
        print(f"[done] normal termination after {scheduler.steps} step(s).", file=sys.stderr)

# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Run a ConMa VProc to completion.",
        add_help=True,
    )
    parser.add_argument(
        "-t", "--trace",
        action="store_true",
        help="Print VProc state to stderr before each step.",
    )
    parser.add_argument(
        "-m", "--main",
        action="store_true",
        help="Start the program with __start__ and pass the remaining arguments.",
    )
    parser.add_argument("module_file", help="Path to the module S-expression file.")
    parser.add_argument("names", nargs="*", metavar="func_name | arg",
                        help="Name of the function to call, or with --main the program arguments.")

    args = parser.parse_args()
    if args.main:
        run(args.module_file, "__start__", trace=args.trace, program_args=args.names)
    elif len(args.names) == 1:
        run(args.module_file, args.names[0], trace=args.trace)
    else:
        parser.error("expected exactly one func_name (or use --main)")

if __name__ == "__main__":
    main()
