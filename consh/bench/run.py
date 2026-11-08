#!/usr/bin/env python3
"""consh/bench/run.py: time the scripts of consh/bench on versions of ConSh and VMs (README-ja.md).

Usage: consh/bench/run.py [-n N] [-v VERSION,...] [script ...]

VERSION is one of
  rust                  the Rust version (consh/rust/target/release/consh-rs)
  conma:rust            the fast ConMa version on the Rust VM (vm/target/release/conma)
  conma:transpiled      the fast ConMa version on the transpiled VM (vm/target-transpiled/release/conma)
  reference:rust        the reference ConMa version on the Rust VM
  reference:transpiled  the reference ConMa version on the transpiled VM
(by default rust,conma:transpiled). Each script (by default, all the scripts NN_*.sh here) is run N times
(3 by default) with each version, and the least user + sys time of the process is printed, including its
start, as a Markdown table with the ratio of each version to the first one; the outputs of the versions
must be the same.

The executables are started as bin/consh starts them, without bin/consh: consh-rs run <script>, and
conma --main <module> run-src <script> <script>. The modules (/tmp/consh.module.se and
/tmp/consh.reference.module.se, or those of CONSH_MODULE_DIR) are made first by bin/consh, which also warns
when consh-rs is older than its sources. The transpiled code of a module is used only if the transpiled VM
was built with that very module (bench/build.sh; CONMA_BENCH_REFERENCE=1 for the reference version), which
is checked before timing (CONMA_TRANSPILED_REPORT).
"""
import argparse
import glob
import os
import resource
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CONSH = os.path.dirname(HERE)
ROOT = os.path.dirname(CONSH)
MODULE_DIR = os.environ.get("CONSH_MODULE_DIR", "/tmp")
VMS = {
    "rust": os.path.join(ROOT, "vm/target/release/conma"),
    "transpiled": os.path.join(ROOT, "vm/target-transpiled/release/conma"),
}
MODULES = {
    "conma": os.path.join(MODULE_DIR, "consh.module.se"),
    "reference": os.path.join(MODULE_DIR, "consh.reference.module.se"),
}
RUST = os.path.join(CONSH, "rust/target/release/consh-rs")


def fail(msg):
    print("run.py: " + msg, file=sys.stderr)
    sys.exit(1)


def command(version):
    """The command that runs a script (to be appended) with the version."""
    if version == "rust":
        if not os.access(RUST, os.X_OK):
            fail(f"{RUST} is not built (cargo build --release in consh/rust)")
        return lambda s: [RUST, "run", s]
    impl, _, vm = version.partition(":")
    if impl not in MODULES or vm not in VMS:
        fail(f"unknown version {version!r} (rust, conma:rust, conma:transpiled, reference:rust, reference:transpiled)")
    if not os.access(VMS[vm], os.X_OK):
        fail(f"{VMS[vm]} is not built")
    return lambda s: [VMS[vm], "--main", MODULES[impl], "run-src", s, s]


def prepare(versions):
    """Make the modules (and let bin/consh warn about an old consh-rs), and check that the transpiled VM
    uses the transpiled code of the modules."""
    empty = os.path.join(HERE, "00_start.sh")
    for v in versions:
        impl = v.partition(":")[0]
        env = dict(os.environ, CONSH_IMPL=impl, CONMA_VM="rust")
        p = subprocess.run([os.path.join(CONSH, "bin/consh"), empty], env=env, capture_output=True, text=True)
        if p.returncode != 0 or p.stderr:
            print(p.stderr, end="", file=sys.stderr)
        if p.returncode != 0:
            fail(f"bin/consh failed for {v}")
        if v.endswith(":transpiled"):
            env = dict(os.environ, CONMA_TRANSPILED_REPORT="1")
            p = subprocess.run(command(v)(empty), env=env, capture_output=True, text=True)
            if "transpiled code: used" not in p.stderr:
                fail(f"the transpiled VM does not have the transpiled code of {MODULES[impl]} "
                     f"(bench/build.sh{' with CONMA_BENCH_REFERENCE=1' if impl == 'reference' else ''})")


def run(cmd):
    """user + sys seconds of the process, and its output."""
    r0 = resource.getrusage(resource.RUSAGE_CHILDREN)
    p = subprocess.run(cmd, cwd=HERE, stdin=subprocess.DEVNULL, capture_output=True)
    r1 = resource.getrusage(resource.RUSAGE_CHILDREN)
    t = (r1.ru_utime - r0.ru_utime) + (r1.ru_stime - r0.ru_stime)
    return t, (p.returncode, p.stdout, p.stderr)


def main():
    ap = argparse.ArgumentParser(description="Time the scripts of consh/bench.")
    ap.add_argument("-n", type=int, default=3, help="runs of each script (the least time is taken)")
    ap.add_argument("-v", default="rust,conma:transpiled", help="versions, separated by commas")
    ap.add_argument("scripts", nargs="*")
    a = ap.parse_args()
    versions = a.v.split(",")
    commands = [command(v) for v in versions]
    # A script is a path, or the name of a script here
    scripts = [os.path.abspath(s if os.path.exists(s) else os.path.join(HERE, s)) for s in a.scripts]
    scripts = scripts or sorted(glob.glob(os.path.join(HERE, "[0-9][0-9]_*.sh")))
    prepare(versions)
    print("| script | " + " | ".join(versions) + " |")
    print("|---" * (len(versions) + 1) + "|")
    differ = False
    for s in scripts:
        times, outs = [], []
        for c in commands:
            best = None
            for _ in range(a.n):
                t, out = run(c(s))
                best = t if best is None else min(best, t)
            times.append(best)
            outs.append(out)
        cells = [f"{times[0]:.3f} s"]
        for t in times[1:]:
            cells.append(f"{t:.3f} s ({t / times[0]:.1f}×)" if times[0] > 0 else f"{t:.3f} s")
        same = all(o == outs[0] for o in outs)
        differ = differ or not same
        print(f"| {os.path.basename(s)} | " + " | ".join(cells) + " |" + ("" if same else " outputs differ"),
              flush=True)
        if not same:
            for v, (code, out, err) in zip(versions, outs):
                print(f"  {v}: status {code}, stdout {out[:200]!r}, stderr {err[:200]!r}", file=sys.stderr)
    sys.exit(1 if differ else 0)


if __name__ == "__main__":
    main()
