#!/usr/bin/env python3
"""consh/bench/run.py: time the scripts of consh/bench on versions of ConSh and VMs (README-ja.md).

Usage: consh/bench/run.py [-n N] [-v VERSION[=PATH],...] [script ...]

VERSION is one of
  rust                  the Rust version (consh/rust/target/release/consh-rs)
  conma:rust            the extended ConMa version on the Rust VM (vm/target/release/conma)
  conma:transpiled      the extended ConMa version on the transpiled VM (vm/target-transpiled/release/conma)
  reference:rust        the reference ConMa version on the Rust VM
  reference:transpiled  the reference ConMa version on the transpiled VM
  conma:wasm            the extended ConMa version on the wasm VM (vm/wasm/vm-wasm.sh, vm/wasm/build.sh)
  reference:wasm        the reference ConMa version on the wasm VM
  bash                  Bash (bash in PATH) with the Bash version of the script, bash/NN_*.sh
(by default rust,conma:transpiled). VERSION=PATH runs the version with another executable: PATH in place of
consh-rs, of bash, of the VM (vm/target/release/conma, vm/target-transpiled/release/conma), or of
vm/wasm/vm-wasm.sh, such as a copy made before rebuilding it, to measure the version before a change beside
the version after it: -v conma:transpiled=/tmp/before/conma,conma:transpiled. A copy of the wasm VM keeps the
layout of the repository: vm-wasm.sh, vm-node.js, vm-jsc.js, conma.js and
target/wasm32-unknown-unknown/release/conma_wasm.wasm of vm/wasm in DIR/vm/wasm, and vm/frontend.module.se in
DIR/vm (vm-node.js reads it from ../.. of its directory). The column of the table is VERSION=PATH as given.
Each script (by default, all the scripts NN_*.sh here) is run N times (3 by default) with each version, the
versions in turn (the first run of each version, then the second, ...), so that a change of the state of the
machine falls on all of them, and the least user + sys time of the process is printed, including its
start, as a Markdown table with the ratio of each version to the first one; the outputs of the versions
must be the same (for bash, only the status and the standard output, as its error messages are not those of
ConSh). The scripts that use (( )) (19 to 21), which the Rust version does not have, are left out of the
default list when rust is among the versions, and the scripts that have no Bash version (19 to 21) when bash is.

The executables are started as bin/consh starts them, without bin/consh: consh-rs run <script>, and
conma --main <module> run-src <script> <script>. The modules (consh.module.se and consh.reference.module.se
in CONSH_MODULE_DIR, or in the directory bin/cache-dir gives) are made first by bin/consh, which also warns
when consh-rs is older than its sources. The transpiled code of a module is used only if the transpiled VM
was built with that very module (bench/build.sh; CONMA_BENCH_REFERENCE=1 for the reference version), which
is checked before timing (CONMA_TRANSPILED_REPORT).
"""
import argparse
import glob
import os
import resource
import shutil
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CONSH = os.path.dirname(HERE)
ROOT = os.path.dirname(CONSH)
MODULE_DIR = os.environ.get("CONSH_MODULE_DIR") or subprocess.run(
    [os.path.join(ROOT, "bin", "cache-dir")], capture_output=True, text=True, check=True).stdout.strip()
VMS = {
    "rust": os.path.join(ROOT, "vm/target/release/conma"),
    "transpiled": os.path.join(ROOT, "vm/target-transpiled/release/conma"),
    "wasm": os.path.join(ROOT, "vm/wasm/vm-wasm.sh"),
}
WASM = os.path.join(ROOT, "vm/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm")
MODULES = {
    "conma": os.path.join(MODULE_DIR, "consh.module.se"),
    "reference": os.path.join(MODULE_DIR, "consh.reference.module.se"),
}
RUST = os.path.join(CONSH, "rust/target/release/consh-rs")
BASH_DIR = os.path.join(HERE, "bash")


def fail(msg):
    print("run.py: " + msg, file=sys.stderr)
    sys.exit(1)


def name(spec):
    """The version of VERSION[=PATH]."""
    return spec.partition("=")[0]


def executable(spec, default):
    """The PATH of VERSION=PATH (absolute, and checked), or default."""
    path = spec.partition("=")[2]
    if not path:
        return default
    path = os.path.abspath(os.path.expanduser(path))
    if not os.access(path, os.X_OK) or os.path.isdir(path):
        fail(f"{path} (in {spec!r}) is not an executable file")
    return path


def command(spec):
    """The command that runs a script (to be appended) with the version VERSION[=PATH]."""
    version = name(spec)
    if version == "rust":
        rust = executable(spec, RUST)
        if not os.access(rust, os.X_OK):
            fail(f"{rust} is not built (cargo build --release in consh/rust)")
        return lambda s: [rust, "run", s]
    if version == "bash":
        bash = executable(spec, shutil.which("bash"))
        if not bash:
            fail("bash is not in PATH")
        return lambda s: [bash, bash_script(s)]
    impl, _, vm = version.partition(":")
    if impl not in MODULES or vm not in VMS:
        fail(f"unknown version {version!r} (rust, bash, and conma or reference with :rust, :transpiled or :wasm)")
    exe = executable(spec, VMS[vm])
    if vm == "wasm":
        wasm = os.path.join(os.path.dirname(exe), os.path.relpath(WASM, os.path.dirname(VMS["wasm"])))
        if not os.path.exists(wasm):
            fail(f"{wasm} is not built (vm/wasm/build.sh)")
    if not os.access(exe, os.X_OK):
        fail(f"{exe} is not built")
    return lambda s: [exe, "--main", MODULES[impl], "run-src", s, s]


def bash_script(script):
    """The Bash version of a script: bash/ with the same name."""
    return os.path.join(BASH_DIR, os.path.basename(script))


def prepare(versions):
    """Make the modules (and let bin/consh warn about an old consh-rs), and check that the transpiled VM
    uses the transpiled code of the modules."""
    empty = os.path.join(HERE, "00_start.sh")
    for v in versions:
        if name(v) == "bash":
            continue
        impl = name(v).partition(":")[0]
        env = dict(os.environ, CONSH_IMPL=impl, CONMA_VM="rust")
        p = subprocess.run([os.path.join(CONSH, "bin/consh"), empty], env=env, capture_output=True, text=True)
        if p.returncode != 0 or p.stderr:
            print(p.stderr, end="", file=sys.stderr)
        if p.returncode != 0:
            fail(f"bin/consh failed for {v}")
        if name(v).endswith(":transpiled"):
            env = dict(os.environ, CONMA_TRANSPILED_REPORT="1")
            p = subprocess.run(command(v)(empty), env=env, capture_output=True, text=True)
            if "transpiled code: used" not in p.stderr:
                fail(f"the transpiled VM of {v} does not have the transpiled code of {MODULES[impl]} "
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
    ap.add_argument("-v", default="rust,conma:transpiled",
                    help="versions (VERSION or VERSION=PATH), separated by commas")
    ap.add_argument("scripts", nargs="*")
    a = ap.parse_args()
    versions = a.v.split(",")
    names = [name(v) for v in versions]
    commands = [command(v) for v in versions]
    # A script is a path, or the name of a script here
    scripts = [os.path.abspath(s if os.path.exists(s) else os.path.join(HERE, s)) for s in a.scripts]
    if not scripts:
        scripts = sorted(glob.glob(os.path.join(HERE, "[0-9][0-9]_*.sh")))
        if "rust" in names:
            arith = [s for s in scripts if "((" in open(s, encoding="utf-8").read()]
            scripts = [s for s in scripts if s not in arith]
            if arith:
                print("run.py: left out, as the Rust version has no (( )): "
                      + " ".join(os.path.basename(s) for s in arith), file=sys.stderr)
        if "bash" in names:
            nobash = [s for s in scripts if not os.path.exists(bash_script(s))]
            scripts = [s for s in scripts if s not in nobash]
            if nobash:
                print("run.py: left out, as they have no Bash version: "
                      + " ".join(os.path.basename(s) for s in nobash), file=sys.stderr)
    elif "bash" in names:
        for s in scripts:
            if not os.path.exists(bash_script(s)):
                fail(f"{s} has no Bash version {bash_script(s)}")
    prepare(versions)
    print("| script | " + " | ".join(versions) + " |")
    print("|---" * (len(versions) + 1) + "|")
    differ = False
    for s in scripts:
        # The versions in turn, so that a change of the state of the machine falls on all of them
        times, outs = [None] * len(commands), [None] * len(commands)
        for _ in range(a.n):
            for i, c in enumerate(commands):
                t, out = run(c(s))
                times[i] = t if times[i] is None else min(times[i], t)
                outs[i] = out
        cells = [f"{times[0]:.3f} s"]
        for t in times[1:]:
            cells.append(f"{t:.3f} s ({t / times[0]:.1f}×)" if times[0] > 0 else f"{t:.3f} s")
        # Bash is compared only by its status and standard output
        full = [o for v, o in zip(names, outs) if v != "bash"]
        same = all(o == full[0] for o in full) and all(
            o[:2] == outs[0][:2] for o in outs)
        differ = differ or not same
        print(f"| {os.path.basename(s)} | " + " | ".join(cells) + " |" + ("" if same else " outputs differ"),
              flush=True)
        if not same:
            for v, (code, out, err) in zip(versions, outs):
                print(f"  {v}: status {code}, stdout {out[:200]!r}, stderr {err[:200]!r}", file=sys.stderr)
    sys.exit(1 if differ else 0)


if __name__ == "__main__":
    main()
