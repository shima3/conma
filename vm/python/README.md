# ConMa VM in Python (the reference)

The first implementation of the ConMa VM. It is kept as the reference for the other VMs, not for running programs: it defines the steps, traces and error messages that the Rust VM (`vm/`), the transpiled VM and the wasm VM must give, and it is much slower than they are. It runs only core ConMa; `bin/vm` translates a module of extended ConMa with `bin/to-core` before running it here.

* `vproc_step.py` — the VM: one step of a VProc (`step`), the primitives, the scheduler (`Scheduler`), loading programs (`load_program`, which runs `bin/includer` and `bin/resolver`) and the S-expressions of VProcs and traces. [docs/dev/VPROC_STEP.md](../../docs/dev/VPROC_STEP.md) describes it.
* `vproc_run.py [--trace] <module_file> <func_name>` and `vproc_run.py [--trace] --main <module_file> [<arg> ...]` — run a module until every VProc has finished.
* `vproc_step.py <module_file>` (as a program) — read a VProc as an S-expression on standard input and write it after one step; `test/vproc_step.sh` steps the VProcs of test/module1 and test/module2 this way.
* `vproc_init.py <module_file> <func_name>` — write the first VProc of a call of a function as an S-expression, for `vproc_step.py` (no test uses it).

The tests start it through `bin/vm`, which uses it when `CONMA_VM` is not set; `test/vm_compare.sh` compares its output and traces with those of the Rust VM. It needs only the standard library of Python 3 (the tests are run with 3.9), and finds `bin/` and `include/` from its own place in the repository.
