# ConMa VM in Rust

A second implementation of the ConMa VM. It reads the same modules (the output of `bin/resolver`) and takes the same arguments as `test/vproc_run.py`. The goal is to give the same output, traces and error messages as `test/vproc_step.py`, step by step, and to be much faster.

## Build and run

```
cd vm && cargo build --release
vm/target/release/conma [--trace] <module_file> <func_name>
vm/target/release/conma [--trace] --main <module_file> [<arg> ...]
```

`test/vm.sh` starts either VM with the arguments of `vproc_run.py`: the Python one by default, the Rust one when `CONMA_VM=rust`. Every `test.sh`, `consh/bin/consh` and the ConSh tests start the VM through it, so

```
CONMA_VM=rust test/step3/test.sh
```

runs a test with the Rust VM. `test/vm_compare.sh <module_file> <func_name>` runs both VMs with `--trace` and shows the differences in the output, the exit code and the trace.

## Stages

The VM is written in the order of the steps of [docs/ROADMAP.md](../docs/ROADMAP.md); a stage is done when the tests of its steps pass with `CONMA_VM=rust`.

| Stage | Steps | Status |
|---|---|---|
| 1. Core: S-expressions, module loading, `step`, traces | 1–3 | done |
| 2. Data primitives: CChain/CFrame, `__Cont_get__`, Number, String, Dict, MSeq, Ref, PDict | 4–12 and later data steps | |
| 3. VProcs and the scheduler, LazyList | | |
| 4. OS: Streams, files, pipes, processes, signals | | |
| 5. ConSh | `consh/test` | |
| 6. Front end and start-up | `selfhost*` | |
| 7. Speed (once the steps match) | | |

## Design

* `sexp.rs` — reading and writing S-expressions. `to_string` writes a list on one line when it fits in 80 characters, as `sexp_to_str` does, so that traces match.
* `module.rs` — loading a module. Every Function and Body is compiled once, when the module is loaded. A global variable that is `Undefined` becomes the primitive of the same name, if there is one.
* `value.rs` — values. They follow the Python classes, including the two forms of a String: a literal of the program keeps its AST node (`StrNode`) and a String made at run time is kept in source form (`Str`), because traces write them differently.
* `step.rs` — one step, as `step` in `vproc_step.py`.
* `sched.rs` — the VProcs and the scheduler.
* `trace.rs` — values and VProcs as S-expressions.
* `prim/` — the primitives, in files by the sections of PRIMITIVES.md. Traces write a primitive by the name of its Python function (`(PrimFunc "prim_OS_print")`).

Memory: values are reference counted (`Rc`). Values without mutable parts cannot form cycles, so cycles can only come through Refs, MSeqs and Dicts; a collector for those cycles will be added with them. Long LLists and LVEnvs are dropped without recursion, and the VM runs in a thread with a large stack, because modules are deeply nested.
