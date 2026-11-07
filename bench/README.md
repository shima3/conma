# Benchmarks: ConMa and CPython

Each benchmark is a small, known Python program and the same computation in ConMa, done in the same order, so that both print the same output:

* `nbody/`: the orbits of the Jovian planets (the n-body benchmark of pyperformance and of the Computer Language Benchmarks Game). `nbody.py N` and `nbody.se` (`main N`) print the energy of the system, advance it N steps of 0.01 days, and print the energy again.

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
