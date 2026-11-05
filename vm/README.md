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
| 2. Data primitives: CChain/CFrame, `__Cont_get__`, Number, String, Dict, MSeq, Ref, PDict, `__Sexp_read__` | 4–8, 12, 18, 19, 28, 30, 34 | done |
| 3. VProcs and the scheduler, LazyList, `__Ref_wait_change__` | 10, 17, 20 | done |
| 4. OS: Streams, files, pipes, processes, signals | 9, 11, 13–16, 21–27, 29, 31–33 | done: every step passes |
| 5. ConSh: every test, and collecting cycles so that ConSh can run for long | `consh/test` | done |
| 6. Front end and start-up | `selfhost1`–`7`, `module1`, `module2` | done: every test passes without changes to the VM (see Start-up) |
| 7. Speed (once the steps match) | | |

## Start-up

Measured on an Apple M4, with the ConSh module already built:

* `consh` runs `echo hi` in 0.06 s, of which `consh_parse` (Python) takes 0.02 s; with `--conma-parser` it takes 0.03 s. Loading the ConSh module (5 MB) takes about 0.02 s and 130 MB.
* Building the ConSh module, when its sources have changed, takes 1.2 s with `bin/includer` and `bin/resolver`. The ConMa front end (`test/selfhost7/test.se`) gives the same module in 9.9 s with the Rust VM, so it is left to stage 7.

## Design

* `sexp.rs` — reading and writing S-expressions. `to_string` writes a list on one line when it fits in 80 characters, as `sexp_to_str` does, so that traces match.
* `module.rs` — loading a module. Every Function and Body is compiled once, when the module is loaded. A global variable that is `Undefined` becomes the primitive of the same name, if there is one.
* `value.rs` — values. They follow the Python classes, including the two forms of a String: a literal of the program keeps its AST node (`StrNode`) and a String made at run time is kept in source form (`Str`), because traces write them differently.
* `step.rs` — one step, as `step` in `vproc_step.py`.
* `vproc.rs` — the state of a VProc. A VProc is held as a `VProcCell`, whose serial number and state are outside the `RefCell` of the rest, so that a primitive can act on its own VProc (suspend it, wait, kill it) while that VProc runs the step.
* `sched.rs` — the scheduler: the ready queue and the VProcs that wait for the OS (a process to exit, a Stream to be readable or writable), for another VProc, for a LazyList or for a Ref to change. `__Ref_wait_change__` waits while the value is the same object, as `is` in Python. Numbers therefore have an identity too: every new Number gets its own id, and ConSh depends on it (it sets a Ref to an equal but new Number to wake the VProcs that wait for it).
* `sys.rs` — the C functions that the standard library does not provide (pipe, dup, poll, signal, kill, sysconf), with the constants of macOS and Linux. The crate has no dependencies.
* `gc.rs` — collecting cycles (see Memory).
* `trace.rs` — values and VProcs as S-expressions.
* `prim/` — the primitives: `basic.rs` (`__is_null__`, `__type_name__`), `os.rs`, `number.rs`, `llist.rs`, `cchain.rs` (CChain, CFrame, `__Cont_get__`), `refs.rs` (Ref, PDict), `collections.rs` (MSeq, Dict), `string.rs` (String, `__Sexp_read__`), `vprocs.rs` (VProc, and loading a program with `bin/includer` and `bin/resolver` for `__VProc_spawn__`), `lazy.rs` (LazyList, `__Ref_wait_change__`), `stream.rs` (Streams, files and directories, the environment), `process.rs` (processes and SIGINT). Traces write a primitive by the name of its Python function (`(PrimFunc "prim_OS_print")`).

Memory: values are reference counted (`Rc`). Values without mutable parts cannot form cycles, so cycles can only come through Refs, MSeqs, Dicts, the PDict, VProcs and GVEnvs (a VProc can hold itself, for example in its LVEnv), and ConSh makes such cycles all the time: without collecting them, a loop of `$( )` kept about 40 KB per iteration. `gc.rs` collects them as the cycle collector of Python does, without knowing the roots: every Ref, MSeq, Dict, VProc and GVEnv is registered when it is made, and a collection finds every value they reach, counts the references to each one from inside that graph, and takes a value with more references (`Rc::strong_count`) as reachable from outside (the scheduler, a primitive, any Rust code). The containers that nothing reachable from outside reaches are emptied, which frees the cycles. A reference that the collector does not see only keeps values alive. Collections run between two steps, after as many containers have been made as survived the last one (at least 10000); `CONMA_GC_THRESHOLD=n` collects after every n containers instead, to test the collector, and `CONMA_GC_STATS=1` prints what each collection freed and kept. The list of VProcs that a deadlock is reported from keeps only the VProcs that have not finished. Long LLists and LVEnvs are dropped without recursion, and the VM runs in a thread with a large stack, because modules are deeply nested.

OS: as in the Python VM, fds made by the VM are not inherited by child processes, a child gets the Streams given to `__OS_spawn__` as its standard streams, and SIGCHLD (and a SIGINT after `__OS_trap_interrupt__`) writes to a pipe, so that the scheduler sleeps in `poll` until a process exits or a Stream is ready. When VProcs wait for the OS while others run, the OS is checked every 64 steps, as in the Python VM; such traces depend on timing, so they may differ from run to run in either VM.
