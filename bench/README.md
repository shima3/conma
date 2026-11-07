# Benchmarks: ConMa and CPython

Each benchmark is a small, known Python program and the same computation in ConMa, done in the same order, so that both print the same output:

* `nbody/`: the orbits of the Jovian planets (the n-body benchmark of pyperformance and of the Computer Language Benchmarks Game). `nbody.py N` and `nbody.se` (`main N`) print the energy of the system, advance it N steps of 0.01 days, and print the energy again.
* `fib/`: the naive recursive Fibonacci function, a benchmark of calls. `fib.py N` and `fib.se` (`main N`) print fib(N).
* `loop/`: a loop that counts N down to 0, a benchmark of the simplest iteration (a function that calls itself in ConMa, `while` in Python). `loop.py N` and `loop.se` (`main N`) print N.
* `richards/`: Martin Richards' simulation of the task dispatcher of an operating system (the richards benchmark of pyperformance). `richards.py N` and `richards.se` (`main N`) run it N times and print the hold count and the packet count of each run (9297 23246). Tasks, packets and the records of the tasks are objects in Python and Dicts in ConMa, a task having the function that runs it under `"fn"`. ConMa has no bit operations, so the idle task computes `control & 1`, `control // 2` and `^ 0xD008` with arithmetic (`xor_bits`, bit by bit).

```
bench/build.sh            # build the transpiled VM with the transpiled code of the benchmarks (and of ConSh)
bench/run.sh nbody 100000 # CPython, the Rust VM and the transpiled VM: best of 3, and a check of the output
```

* The Python versions are pure Python (no C extension but `math.sqrt`).
* CPython is an interpreter of bytecode, like the Rust VM. The transpiled VM runs code made from the ConMa program by the ConMa-to-Rust transpiler (`vm/conma2rust`) ahead of time, as Cython or Nuitka do for Python. The goal for now is a transpiled VM about as fast as CPython.
* `bench/module.sh` writes the module of a benchmark with paths starting with `conma/`, so that its text, and so the transpiled code built for it, does not depend on where the repository is.

## Results

Apple M4, CPython 3.9.6, times in user + sys seconds (best of 3), including the start of each program.

| Benchmark | CPython | Rust VM | Transpiled VM |
|---|---|---|---|
| nbody 100000 | 0.56 s | 6.59 s (11.8×) | 3.80 s (6.8×) |
| richards 20, when it was added | 0.81 s | about 10.4 s (13×) | 1.99 s (2.5×) |

`bench/run.sh` leaves the Rust VM out unless `CONMA_BENCH_RUST=1`. The transpiler has since been improved (vm/README.md, "Fused chains"); the tables below give the effect of each change on the transpiled VM.

### Effect of each change: up to calls with an LCont

| nbody 100000, transpiled VM | Time | CPython = 1 |
|---|---|---|
| before | 3.80 s | 6.8 |
| fused chains (variables in Rust variables) | 1.61 s | 2.9 |
| + Dict index by `&str`, Fx hash | 1.24 s | 2.2 |
| + operands of direct primitives by reference | 1.08 s | 1.9 |
| + Numbers as `f64` | 0.91 s | 1.55 |
| + String keys written in place looked up by their text | 0.72 s | 1.24 |
| + calls with an LCont as Rust calls in fused chains | 0.67 s | 1.18 |
| + `onMissing` evaluated only when the key is missing, `chk` inlined | 0.51 s | 0.89 |

### Effect of each change: since then

The transpiled VM after each change, built with the transpiled code of the benchmarks and timed together with CPython (user + sys, best of 5). All of them print the same. ConSh: the 15,000 function calls of `for i in $(seq 1 15000); do add $i x > (r); done` (user time, measured when the change was made, with the transpiled code of ConSh).

| Change | Date | nbody 100000 | richards 20 | fib 32 | loop 10000000 | ConSh |
|---|---|---|---|---|---|---|
| before: calls with an LCont as Rust calls, `__Cont_get__` and `__if__` with an LCont in fused chains | 2026-11-07 | 0.49 s | 2.10 s | 0.68 s | 0.65 s | 0.89 s |
| a function that calls itself without an LCont as a Rust loop | 2026-11-07 | 0.40 s | 1.98 s | 0.68 s | 0.12 s | 0.92 s |
| arguments passed to a direct entry of the function called, not through an LVEnv | 2026-11-07 | 0.38 s | 1.77 s | 0.48 s | 0.12 s | 0.90 s |
| `__Number_floor__` computed on `f64` in fused chains | 2026-11-07 | 0.37 s | 1.19 s | 0.48 s | 0.12 s | — |
| last uses of variables moved, not cloned; Booleans of `__is_null__` and the Number predicates as Rust `bool`s | 2026-11-07 | 0.35 s | 1.03 s | 0.39 s | 0.06 s | 0.81 s |
| values returned in an array, straight back to the Rust call | 2026-11-07 | 0.35 s | 1.00 s | 0.36 s | 0.06 s | 0.80 s |
| hashes of String keys written in place computed at compile time | 2026-11-07 | 0.30 s | 0.87 s | 0.35 s | 0.06 s | 0.81 s |
| a Rust call ended without `drive` and, with no continuation made in it, without looking for CFrames | 2026-11-08 | 0.29 s | 0.82 s | 0.31 s | 0.06 s | 0.81 s |
| a value read from a Dict with a String key written in place and used only as conditions: a Rust `bool`, tested in the Dict without cloning it | 2026-11-08 | 0.29 s | 0.78 s | 0.32 s | 0.06 s | 0.79 s |
| String keys in the index of a Dict by their hash, a short text compared without `memcmp` | 2026-11-08 | 0.25 s | 0.77 s | 0.31 s | 0.06 s | 0.80 s |
| CPython 3.9.6 | | 0.56 s | 0.79 s | 0.35 s | 0.31 s | |

So the transpiled VM is now faster than CPython 3.9.6 for nbody, fib and the loop, and as fast for richards (slightly faster), whose idle task computes its bit operations with arithmetic (ConMa has none).

What the changes for richards were found from (samples of the running VM): at first, calls took about 40% of its time (their machinery, `drive`, `frame_run`, `fused_call`, 23%, and the LVEnv nodes the arguments go into, 16%), cloning and dropping values 27%, and the Dict lookups 12%; and the bit operations about a third (computing each `^ 0xD008` twice made the run 0.63 s longer), because `__Number_floor__` went back to the code of the Bodies at each bit, making and dropping an LVEnv of 12 variables. After the changes above, the time goes to the call machinery (about 26%), cloning and dropping values (26%), the Dict lookups and puts (21%, a fourth of it comparing the texts of keys) and the generated code itself (24%).

### What these results do not say

They are for these programs: no implementation is faster than another for every program. The cost of each kind of operation differs between the two, and a program made mostly of operations that one does better is faster there:

* ConMa has no bit operations: they are written with arithmetic (as in richards), several times slower than an operator of Python.
* Much of what a Python program does runs in C inside CPython (`sort`, the methods of `str`, `dict` and `list`), while the same work in ConMa is ConMa code.
* Some operations take time that grows differently: `__Dict_remove__` rebuilds the index of the Dict (linear in its size; `del d[k]` in Python takes constant time on average), and `__LList_append__` copies its first list and `__LList_length__` walks the list (linear, as in Lisp; a Python list appends and gives its length in constant time). A growing sequence is an MSeq in ConMa.
* The transpiled VM is fast only where the transpiled code runs as expected: a module without transpiled code (any module not built into the VM, `vm/conma2rust/build.sh`), and the Bodies a fused chain hands back to the code of the Bodies, run at the speed of the Rust VM.
* The comparison is with CPython 3.9.6; later versions of CPython run pure Python code faster.
