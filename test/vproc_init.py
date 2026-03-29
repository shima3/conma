#!/usr/bin/env python3
"""vproc_init.py — Build the initial VProc state for a named function call.

Usage:
    vproc_init.py <module_file> <func_name>

Reads the module S-expression from <module_file>, resolves the function
named <func_name>, and prints the initial VProc state as an S-expression
to stdout.  The output can be piped directly into vproc_step.py:

    vproc_init.py module.sexp case1 | vproc_step.py module.sexp
"""

import sys, re

# ── Re-use helpers from vproc_step (copy-free: just import the module) ────────
# If vproc_step.py is on the path we import it; otherwise we inline what we need.

try:
    import importlib.util, pathlib

    # Try to find vproc_step.py next to this script first, then cwd.
    for candidate in [
        pathlib.Path(__file__).parent / "vproc_step.py",
        pathlib.Path("vproc_step.py"),
        pathlib.Path("vproc_step8.py"),
        pathlib.Path(__file__).parent / "vproc_step8.py",
    ]:
        if candidate.exists():
            spec = importlib.util.spec_from_file_location("vproc_step", candidate)
            vproc_step = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(vproc_step)
            break
    else:
        raise ImportError("vproc_step.py not found")

    parse_sexp     = vproc_step.parse_sexp
    sexp_to_str    = vproc_step.sexp_to_str
    load_module    = vproc_step.load_module
    value_to_sexp  = vproc_step.value_to_sexp
    unquote        = vproc_step.unquote
    quoted         = vproc_step.quoted
    VProcError     = vproc_step.VProcError

except ImportError as e:
    print(f"Error: {e}", file=sys.stderr)
    sys.exit(1)


# ── Name → GID index ──────────────────────────────────────────────────────────

def build_name_index(module_sexp):
    """Return a dict mapping unquoted function names to their integer GIDs."""
    index = {}
    for item in module_sexp[2:]:
        if not isinstance(item, list) or item[0] != "G":
            continue
        gid  = int(item[1])
        name = unquote(item[2]) if len(item) > 2 else None
        if name:
            index[name] = gid
    return index


# ── Initial VProc builder ─────────────────────────────────────────────────────

def make_initial_vproc(operator_sexp, module_file):
    """Return an S-expression list representing an empty initial VProc state."""
    return [
        "__VProc__",
        ["__SInfo__"],
        ["__Operator__", operator_sexp],
        ["__OList__"],
        ["__LCont__"],
        ["__LVEnv__"],
        ["__GVEnv__", quoted(module_file)],
        ["__CChain__"],
        ["__PDict__"],
    ]


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) != 3:
        print("usage: vproc_init.py <module_file> <func_name>", file=sys.stderr)
        sys.exit(1)

    module_file = sys.argv[1]
    func_name   = sys.argv[2]

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

    gid  = name_index[func_name]
    gvenv = load_module(module_sexp)

    closure = gvenv.get(gid)
    if closure is None:
        print(f"Error: GID {gid} ({func_name!r}) is undefined in gvenv.", file=sys.stderr)
        sys.exit(1)

    operator_sexp = value_to_sexp(closure)
    vproc_sexp    = make_initial_vproc(operator_sexp, module_file)

    print(sexp_to_str(vproc_sexp))


if __name__ == "__main__":
    main()
