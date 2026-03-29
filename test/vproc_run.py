#!/usr/bin/env python3
"""vproc_run.py — Run a ConMa VProc to completion.

Usage:
    vproc_run.py [options] <module_file> <func_name>

Options:
    -t, --trace     Print the VProc state to stderr before each step.
    -h, --help      Show this message and exit.

Exit codes:
    0   Normal termination (Null operator with empty CChain and no LCont).
    1   Runtime error (VProcError or I/O failure).
"""

import sys, pathlib, importlib.util, argparse

# ── Import vproc_step ─────────────────────────────────────────────────────────

def _load_vproc_step():
    for candidate in [
        pathlib.Path(__file__).parent / "vproc_step.py",
        pathlib.Path("vproc_step.py"),
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
step          = vs.step
unquote       = vs.unquote
quoted        = vs.quoted
is_null_value = vs.is_null_value
VProcError    = vs.VProcError

# ── Name index ────────────────────────────────────────────────────────────────

def build_name_index(module_sexp):
    index = {}
    for item in module_sexp[2:]:
        if not isinstance(item, list) or item[0] != "G":
            continue
        gid  = int(item[1])
        name = unquote(item[2]) if len(item) > 2 else None
        if name:
            index[name] = gid
    return index

# ── Initial VProc ─────────────────────────────────────────────────────────────

def make_initial_vproc(closure, module_file, gvenv):
    return {
        "sinfo":       None,
        "operator":    closure,
        "olist":       [],
        "lcont":       None,
        "lvenv":       [],
        "gvenv_files": [module_file],
        "cchain":      [],
        "pdict":       [],
    }

# ── Termination check ─────────────────────────────────────────────────────────

def is_terminal(vp):
    """True when the VProc has reached a normal halting state."""
    return (
        is_null_value(vp["operator"])
        and vp["lcont"] is None
        and not vp["cchain"]
    )

# ── Trace helper ──────────────────────────────────────────────────────────────

def print_trace(vp, step_n):
    label = f"[step {step_n}]" if step_n > 0 else "[initial]"
    print(f"{label} {sexp_to_str(vproc_to_sexp(vp))}", file=sys.stderr)

# ── Runner ────────────────────────────────────────────────────────────────────

def run(module_file, func_name, trace=False):
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

    vp     = make_initial_vproc(closure, module_file, gvenv)
    step_n = 0

    if trace:
        print_trace(vp, step_n)

    try:
        while not is_terminal(vp):
            step(vp, gvenv)
            step_n += 1
            if trace:
                print_trace(vp, step_n)
    except VProcError as e:
        print(f"Error at step {step_n}: {e}", file=sys.stderr)
        sys.exit(1)

    if trace:
        print(f"[done] normal termination after {step_n} step(s).", file=sys.stderr)

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
    parser.add_argument("module_file", help="Path to the module S-expression file.")
    parser.add_argument("func_name",   help="Name of the function to call.")

    args = parser.parse_args()
    run(args.module_file, args.func_name, trace=args.trace)

if __name__ == "__main__":
    main()
