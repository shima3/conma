# Benchmarks: ConMa and CPython

Each benchmark is a small, known Python program and the same computation in ConMa, done in the same order, so that both print the same output:

* `nbody/`: the orbits of the Jovian planets (the n-body benchmark of pyperformance and of the Computer Language Benchmarks Game). `nbody.py N` and `nbody.se` (`main N`) print the energy of the system, advance it N steps of 0.01 days, and print the energy again.
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

The transpiler has since been improved for nbody (vm/README.md, "Fused chains"; `bench/run.sh` now leaves the Rust VM out unless `CONMA_BENCH_RUST=1`):

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
| + calls of a function to itself without an LCont as Rust loops | 0.43 s | 0.72 |
| + arguments passed to a direct entry of the called function, not through an LVEnv | 0.39 s | 0.70 |

With these, the transpiled VM runs nbody faster than CPython 3.9.6 (0.39 s against 0.56 s, measured together).

richards 20 (Apple M4, best of 3; the Rust VM with 5 runs, scaled):

| richards 20 | Time | CPython = 1 |
|---|---|---|
| CPython 3.9.6 | 0.81 s | 1 |
| transpiled VM | 1.99 s | 2.5 |
| transpiled VM, arguments passed to a direct entry | 1.79 s | 2.2 |
| + `__Number_floor__` computed in place in fused chains | 1.21 s | 1.5 |
| Rust VM | about 10.4 s | about 13 |

Where the time of the transpiled VM goes (a sample of the running VM): calls take about 40% (their machinery, `drive`, `frame_run`, `fused_call`, 23%, and the LVEnv nodes the arguments go into, 16%), cloning and dropping values 27% (largely the arguments, into the LVEnv and out of it), and the Dict lookups 12%. The bit operations written with arithmetic take about a third of the time: computing each `^ 0xD008` twice makes the run 0.63 s longer. `__Number_floor__` is not computed in place in fused chains, so each bit goes through the code of the Bodies. With the arguments passed to a direct entry, most of the LVEnv work left was in `xor_bits`, which went back to the code of the Bodies at each `__Number_floor__` and so made and dropped an LVEnv of 12 variables for each bit; fused chains now compute `__Number_floor__` on `f64`, and `xor_bits` is a Rust loop.
