# ConMa VM in Rust

A second implementation of the ConMa VM. It reads the same modules (the output of `bin/resolver`) and takes the same arguments as `test/vproc_run.py`. The goal is to give the same output, traces and error messages as `test/vproc_step.py`, step by step, and to be much faster.

## Build and run

```
cd vm && cargo build --release
vm/target/release/conma [--trace] <module_file> <func_name>
vm/target/release/conma [--trace] --main <module_file> [<arg> ...]
```

It needs Rust 1.75 or later (`rust-version` in `Cargo.toml`), and no crates. ConSh run directly in Rust (`consh/rust`, `CONSH_IMPL=rust`) needs Rust 1.87 or later, for `std::io::pipe`. Both `Cargo.lock` files are in version 3, which Cargo before 1.78 can read.

`test/vm.sh` starts either VM with the arguments of `vproc_run.py`: the Python one by default, the Rust one when `CONMA_VM=rust`. Every `test.sh`, `consh/bin/consh` and the ConSh tests start the VM through it, so

```
CONMA_VM=rust test/step3/test.sh
```

runs a test with the Rust VM. `test/vm_compare.sh <module_file> <func_name>` runs both VMs with `--trace` and shows the differences in the output, the exit code and the trace. With `CONMA_SEED=<n>`, both VMs choose the VProc to run next with the same pseudo-random numbers (see "Scheduling and Termination" in docs/DESIGN.md), so `vm_compare.sh` also compares them under a seed.

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
| 7. Speed (once the steps match) | every test, and the same traces | first round done (see Speed) |

## Start-up

Measured on an Apple M4, with the ConSh module already built:

* `consh` runs `echo hi` in 0.06 s, of which `consh_parse` (Python) takes 0.02 s; with `--conma-parser` it takes 0.03 s. Loading the ConSh module (5 MB) takes about 0.02 s and 130 MB.
* Building the ConSh module, when its sources have changed, takes 1.2 s with `bin/includer` and `bin/resolver`. The ConMa front end (`test/selfhost7/test.se`) gives the same module in 3.7 s with the Rust VM (9.9 s before stage 7).

## Speed

Measured with the profiler of macOS (`sample`) on the ConMa front end building the ConSh module, and then on ConSh. Two changes took most of the time away:

* The allocator. A step makes and drops small Rc blocks (LVNodes, Closures, CFrames, LList cells), and the allocator of macOS took about half of the time (it also clears each block it frees). `alloc.rs` keeps blocks of up to 256 bytes in a free list for each multiple of 16 bytes, carved from chunks of 256 KB, so a block is reused by the next one of the same size. Larger blocks go to the system allocator. Small blocks are never given back to the system, so the memory of a process stays at its peak.
* Strings. A String made at run time was kept in source form, so every String primitive unescaped its arguments and escaped its result: the lexer, which takes a line apart one character at a time, spent about 40% of its time there. Such a String now keeps its text.

Smaller ones: `load_body` reuses the Vec of the OList, and the collector checks plain counters at each step.

| Run | Before | After |
|---|---|---|
| ConMa front end, the ConSh module | 9.9 s | 3.7 s |
| ConSh, `x=$(f $i)` 3000 times | 3.1 s | 1.7 s |
| ConSh, `x=$(f $i)` 20000 times | 21.0 s | 11.1 s |
| ConSh, `echo $i \| cat` 3000 times (a process each time) | 9.9 s | 8.9 s |

Every test passes with `CONMA_VM=rust`, and the traces of steps 1–8, 10, 12, 17–20, 28, 30 and 34 (129 runs) are the same as those of the Python VM. The time is now spread over the interpreter itself (`step`, the scheduler, cloning and dropping values).

Later measurements, on ConSh loops: a local variable is looked up at a mean depth of about 2 in its LVEnv (at most 16 to 40), so an LVEnv kept as arrays of frames would not make lookups faster; and keeping the values of global variables in the compiled expressions saved nothing, because the time goes to cloning the value (the reference count of the Closure), not to finding it. What helped is that the scheduler runs the steps of a VProc one after another while no other VProc is ready and none waits (no trapped SIGINT, no collection due, no kill pending), instead of putting it back in the ready queue after each step: this is what a pass of the scheduler would do anyway, and it saves about 10% of the time of a ConSh loop that runs in one VProc. A loop with `$( )` or pipes has VProcs waiting for channels, so it is not helped.

While a VProc is ready and others wait for the OS, the scheduler checks the OS every 64 steps (`POLL_INTERVAL`, as in the Python VM), and a check (`poll()` and `waitpid()`) took about 7 µs: in `for i in $(seq 1 20000); do n=$i; done`, where a VProc waits for `seq` to end, the 155,250 checks took 1.1 s of the 1.85 s, mostly in the kernel or descheduled. A process that waits can only have ended if a SIGCHLD came, so while only processes wait, a check now does nothing unless the SIGCHLD handler has counted a signal since the last check, or a process was added to the waiting list since then (it may have ended before the handler was installed). The VProcs wake at the same checks as before, so the steps are the same. That loop now takes 0.61 s, `echo $i` 20000 times 1.29 s instead of 3.27 s, and a function called 20000 times 1.87 s instead of 4.95 s.

To see where a ConMa program spends its steps, run it with `CONMA_PROFILE=<file>`: each step is counted under the Body that the VProc is running, and the calls of each Body and the steps that apply each primitive are counted too. `python3 test/vm_profile.py <file>` sums them by file, by top-level `define` (steps, calls and steps per call) and by primitive; `--callers NAME` shows the lines that call a function, and `--thunks` the lines that pass `__if__`, `__unless__` or `__if_then_else__` a condition written as a function instead of a Boolean. The counts do not depend on the speed of the VM, so they also tell how a change to a ConMa program (such as ConSh) would speed up the Python VM.

## Transpiled code (experimental)

`vm/conma2rust/conma2rust.py`, the ConMa-to-Rust transpiler, makes Rust code for ConMa modules (files made by `bin/resolver`), and `vm/conma2rust/build.sh <module>...` builds the VM with it in `vm/target-transpiled`; `CONMA_VM=transpiled` in `test/vm.sh` runs that VM. The transpiled code of a module is used only for a module file with the same text (FNV-1a hash); `CONMA_TRANSPILED_REPORT=1` says whether it is used, and `CONMA_TRANSPILED_OFF=1` turns it off.

For each Function whose Body can run without the VM (a primitive that only works on its operands, `__NULL__` passing values on, `__if_then_else__`, `__if__` and `__unless__` with clauses written in place), the transpiled code runs that Body and the ones after it. Where a Body needs the VM, it is loaded, and `run` (`src/transpiled.rs`) goes on with the steps after it as the VM would, as long as they are calls (a Closure with as many operands as parameters), returns (null as the Operator), escape continuations or safe primitives, entering transpiled code where it can; anything else (a primitive that may block or switch VProcs, a partial application) is left to the VM, and so is the VProc after 1,000 such steps, so that the other VProcs run. All this is one step of the VM. The results are those of the VM; the number of steps, and so the order in which VProcs run, differ, so traces and the step numbers in error messages differ, and tests that depend on that order may give another of the possible results (`test/step20` case11 and case21). With `--trace` or `CONMA_PROFILE` the transpiled code is not used.

The most used primitives (`__is_null__`, `__String_equal__`, `__LList_uncons__`, `__Dict_lookup_or__`, the Number arithmetic and so on; `DIRECT` in `conma2rust.py`) are called directly, with their operands in Rust variables and their results bound without the OList, through functions shared with the primitives (`is_null_core`, `equal_core` and so on), and `__Cont_get__ ,(return)` makes the continuation in place.

**Fused chains.** For each top-level Function, each function expression, and each LCont or clause where the code above hands the VProc back (see below), the transpiler also writes one Rust function for the Body and the Bodies after it (`Fuser` in `conma2rust.py`). The variables these Bodies bind are Rust variables, not LVEnv nodes; Numbers that go only between Number primitives (`__Number_add__`, `__Number_subtract__`, `__Number_multiply__`, `__Number_divide__`, `__Number_sqrt__` and the predicates) are `f64` and computed with Rust arithmetic, an operand that is not a Number giving the error of the primitive; the operands of the direct primitives are passed by reference; `__Dict_lookup_or__` and `__Dict_put__` with a String key written in place (without escapes) look it up by its text, which the transpiler writes into the code (`d_dict_lookup_str`, `d_dict_put_str`). `__NULL__`, and `__if_then_else__`, `__if__` and `__unless__` without an LCont whose clauses are written in place (both inlined as a Rust `match`), are run in place too, and a chain ends with a return or a call without an LCont (left to `apply`, which counts it for the budget). A call with an LCont is a Rust call (below), after which the chain goes on with its variables. `__Cont_get__ ,(return)` makes the continuation in place, in a Rust variable, and `return v` is a call without an LCont. `__if_then_else__`, `__if__` and `__unless__` with an LCont have their clauses inlined in a labeled Rust block: what a clause returns is the value of the block, and the LCont goes on after it (so it is written once). The VM would have the CFrames of the LConts of the clauses the code is in on the CChain; wherever the code hands the VProc back inside a clause (a bail, an `onError`, a call left to the VM), it puts them there first, and a Rust call from a clause puts them there if it is left (`fused_call` and `frame_run` take a function that builds the CFrames, `cframe_on`); a call without an LCont in a clause is a call whose LCont is the clause's, except a continuation (`return`), which drops the LCont. A `__Cont_get__` in a clause is left to the VM (its continuation would lack those CFrames). With these, 560 of the 645 top-level functions of ConSh are fused from their first Body (319 start with `__Cont_get__`), and its 15,000 function calls take 0.91 s instead of 0.98 s. At any other Body, or when a result is not what the chain expects (an `onError`, a Boolean that is not one), it makes the LVEnv the VM would have from the LVEnv it started with and its variables, and enters the transpiled code of that Body above ("bail"); so the results are those of the VM. `vp.lvenv`, which is only shown in traces, is the LVEnv the chain started with. The `onMissing` of `__Dict_lookup_or__` with a String key written in place, and the `onError` of `__Number_divide__` and `__Number_sqrt__` computed on `f64`, are evaluated only when they are applied: in a program where such an operand is an undefined variable, the VM stops with an error at that Body, and the transpiled code only when the operand is needed.

**Dicts.** The index of a Dict keeps String keys by their text in a map that is looked up with a borrowed `&str` (`DictIndex`, `index_key`), so that a lookup does not make a String, and both of its maps use an Fx hash instead of SipHash (Dicts do not need protection against keys made to collide).

A call written as `f a ... ,(x ...) ...` (f a variable), and a clause of `__if__` and the others followed by an LCont, is run as a Rust call when f is a Closure of the module with transpiled code and as many operands as parameters: the LCont is not made into a Closure and pushed to the CChain, but stays in the Rust code (`rust_call`), which binds the values the call returns to it; a call without an LCont (a tail call) goes on in place. A Rust call returns when the VProc passes values to the CChain it was called with (null as the Operator, or a continuation whose CChain is that one). Where the VProc must leave transpiled code (a step of the VM, the end of the budget, a continuation made out of the Rust calls), each Rust call puts the CFrame the VM would have pushed into the CChain on the way out, so that the VM goes on with the same CChain; a continuation made in a Rust call around the current one is applied by that one, leaving the ones in between. A continuation made in a Rust call has a CChain without the CFrames of the Rust calls around it; each Rust call records the continuations made in it, and those still in use when it ends get its CFrame then (`ContObj::cchain` can change for that). Rust calls deeper than 256 are made as the VM makes them, and each call counts as a step for the budget. The Boolean Closures are applied in place too. Alone, this gained little (making the LCont Closure and the CFrame was not where the time went), but it lets fused chains (above) call a function and go on with their Rust variables: a call with an LCont in a fused chain is a Rust call (`fused_call`, sharing `frame_run` with `rust_call`), and the chain binds the values it returns; the LVEnv of the LCont is made only if the call is left or a continuation made in it outlives it. A Rust call that is left costs more than a call made as the VM makes it, so the transpiled code counts, for each Function, how often its Rust calls returned values and how often they were left (to the VM, or by an escape); a Function left more than 8 times, and more than a third as often as it returned values, is called as the VM calls it from then on (`Table::rust_ok`). ConSh reaches a step of the VM (reading a LazyList, setting a Ref) through many calls for every command, and its Rust calls made it about 10% slower than without them (15,000 function calls: 1.08 s against 0.98 s); with this rule it takes 1.00 s, while nbody, whose Functions are never left, keeps them (0.52 s, 0.55 s without Rust calls). A Rust call from a fused chain gets its operands in an array instead of a Vec, and the chain binds the values from the OList, which keeps its buffer; with this, ConSh takes 0.98 s, as before Rust calls, and nbody 0.49 s.

Measurements (Apple M4): a loop of 10,000,000 iterations takes 1.99 s instead of 3.76 s; ConSh (with `/tmp/consh.module.se`) is 1.5 to 1.6 times as fast, and its tests pass. The time now goes mostly to the values themselves (cloning and dropping them, LVEnv nodes, Closures, the cycle collector), not to the steps; much of the cycle collector's work for ConSh came from cycles that ConSh made by mistake for every command, which it no longer makes (consh/PLAN-ja.md), so that ConSh with transpiled code now runs a loop of 15,000 function calls in 1.90 s instead of 3.29 s.

## Design

* `sexp.rs` — reading and writing S-expressions. `to_string` writes a list on one line when it fits in 80 characters, as `sexp_to_str` does, so that traces match.
* `module.rs` — loading a module. Every Function and Body is compiled once, when the module is loaded. A global variable that is `Undefined` becomes the primitive of the same name, if there is one.
* `value.rs` — values. They follow the Python classes, including the two forms of a String: a literal of the program keeps its AST node (`StrNode`) and a String made at run time keeps its text (`Str`), which is escaped and quoted only when a trace writes it.
* `step.rs` — one step, as `step` in `vproc_step.py`.
* `vproc.rs` — the state of a VProc. A VProc is held as a `VProcCell`, whose serial number and state are outside the `RefCell` of the rest, so that a primitive can act on its own VProc (suspend it, wait, kill it) while that VProc runs the step.
* `sched.rs` — the scheduler: the ready queue and the VProcs that wait for the OS (a process to exit, a Stream to be readable or writable), for another VProc, for a LazyList or for a Ref to change. `__Ref_wait_change__` waits while the value is the same object, as `is` in Python. Numbers therefore have an identity too: every new Number gets its own id, and ConSh depends on it (it sets a Ref to an equal but new Number to wake the VProcs that wait for it).
* `sys.rs` — the C functions that the standard library does not provide (pipe, dup, poll, signal, kill, sysconf), with the constants of macOS and Linux. The crate has no dependencies.
* `gc.rs` — collecting cycles (see Memory).
* `alloc.rs` — the global allocator: free lists for blocks of up to 256 bytes (see Speed).
* `profile.rs` — step counts for `CONMA_PROFILE` (see Speed).
* `trace.rs` — values and VProcs as S-expressions.
* `prim/` — the primitives: `basic.rs` (`__is_null__`, `__type_name__`), `os.rs`, `number.rs`, `llist.rs`, `cchain.rs` (CChain, CFrame, `__Cont_get__`), `refs.rs` (Ref, PDict), `collections.rs` (MSeq, Dict), `string.rs` (String, `__Sexp_read__`), `vprocs.rs` (VProc, and loading a program with `bin/includer` and `bin/resolver` for `__VProc_spawn__`), `lazy.rs` (LazyList, `__Ref_wait_change__`), `stream.rs` (Streams, files and directories, the environment), `process.rs` (processes and SIGINT). Traces write a primitive by the name of its Python function (`(PrimFunc "prim_OS_print")`).

Memory: values are reference counted (`Rc`). Values without mutable parts cannot form cycles, so cycles can only come through Refs, MSeqs, Dicts, the PDict, VProcs and GVEnvs (a VProc can hold itself, for example in its LVEnv), and ConSh makes such cycles all the time: without collecting them, a loop of `$( )` kept about 40 KB per iteration. `gc.rs` collects them as the cycle collector of Python does, without knowing the roots: every Ref, MSeq, Dict, VProc and GVEnv is registered when it is made, and a collection finds every value they reach, counts the references to each one from inside that graph, and takes a value with more references (`Rc::strong_count`) as reachable from outside (the scheduler, a primitive, any Rust code). The containers that nothing reachable from outside reaches are emptied, which frees the cycles. A reference that the collector does not see only keeps values alive. Collections run between two steps, after as many containers have been made as survived the last one (at least 10000); `CONMA_GC_THRESHOLD=n` collects after every n containers instead, to test the collector, and `CONMA_GC_STATS=1` prints what each collection freed and kept. The list of VProcs that a deadlock is reported from keeps only the VProcs that have not finished. Long LLists and LVEnvs are dropped without recursion, and the VM runs in a thread with a large stack, because modules are deeply nested.

OS: as in the Python VM, fds made by the VM are not inherited by child processes, a child gets the Streams given to `__OS_spawn__` as its standard streams, and SIGCHLD (and a SIGINT after `__OS_trap_interrupt__`) writes to a pipe, so that the scheduler sleeps in `poll` until a process exits or a Stream is ready. When VProcs wait for the OS while others run, the OS is checked every 64 steps, as in the Python VM; such traces depend on timing, so they may differ from run to run in either VM.
