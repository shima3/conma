# Primitive Implementation Roadmap

This document gives an order for implementing (or re-implementing) the built-in primitives defined in [PRIMITIVES.md](PRIMITIVES.html).
Each step depends only on the steps before it, so the primitives in a step can be tested with what is already available.
The test directory for a step is `test/stepN`. Existing step directories are `step1` to `step10`.

Status shows whether `test/vproc_step.py` currently has the primitive (as of 2026-10-20).

## Tests

Each `test/stepN/test.sh` builds `test.se` with `test/resolver.sh`, runs each case function with `test/vproc_run.py`, and compares the output with `caseN.log`. From `step2` on, `exit: <code>` is appended to the output; `step1` compares the full `--trace` output instead. No output means every case passed.

```
for t in module1 module2 step1 step2 step3 step4 step5 step6 step7 step8 step9 step10; do test/$t/test.sh; done
```

* Standard output and standard error go to the same log, so `PYTHONUNBUFFERED=1` keeps their order.
* Standard input is empty (`/dev/null`) unless a case needs it (`test/step9` case5 reads `input.txt`).
* Cases that run several VProcs make their output independent of how the scheduler interleaves them: a VProc writes only after it has synchronized with the others through Refs and `__VProc_suspend__` / `__VProc_resume__`.

## Step 1: VM foundation

| Primitive | Status |
|---|---|
| `__NULL__` | implemented |
| `__is_null__` | implemented |
| `__OS_print__` | implemented |

* Goal: Check operator application, surplus arguments, partial application and LCont handling (see [DESIGN.md](DESIGN.html)).
* `__OS_print__` is needed first because every later test reports its result through standard output.
* Booleans and control flow (`__TRUE__`, `__FALSE__`, `__if_then_else__`, ...) are defined in `include/pure.se` on top of `__is_null__`, so they need no primitives.

## Step 2: Termination and error output

| Primitive | Status |
|---|---|
| `__OS_print_error__` | implemented |
| `__OS_exit_normal__` | implemented |
| `__OS_exit_error__` | implemented |

* Goal: Fix the `onError` convention before any primitive that takes `onError`.
* After this step `__print_exit_error__` in `include/util.se` can be used as an `onError` handler.

## Step 3: LList

| Primitive | Status |
|---|---|
| `__LList_cons__` | implemented |
| `__LList_uncons__` | implemented |

* `__LList_uncons__` is the first primitive that takes `onError`, so it tests the convention from Step 2.
* This must come before Step 4, because a CChain is traversed as an LList of CFrames (`CChain_print` in `test/step4`, `__Cont_get__` in `include/util.se`).

## Step 4: CFrame & CChain

| Primitive | Status |
|---|---|
| `__CChain_get__` | implemented |
| `__CFrame_get_SInfo__` | implemented |
| `__CFrame_get_Closure__` | implemented |
| `__CChain_set__` | implemented |
| `__CChain_pop_CFrame__` | implemented |
| `__CChain_push_CFrame__` | implemented |
| `__CFrame_new__` | implemented |

* Implement in the order above: first read the CChain, then replace it, then pop and push individual CFrames.
* `__CFrame_new__` takes its SInfo as a colon-separated String, the same form `__CFrame_get_SInfo__` returns (for example `"file.se:12:5"`), or null for no SInfo.
* Library definitions that become available: `__Cont_get__` (escape continuation), `__CChain_pop_LCont__`, `__noop__`.

## Step 5: Number

| Primitive | Status |
|---|---|
| `__Number_toString__` | implemented |
| `__Number_fromString__` | implemented |
| `__Number_is_zero__`, `__Number_is_positive__`, `__Number_is_negative__` | implemented |
| `__Number_add__`, `__Number_subtract__`, `__Number_multiply__` | implemented |
| `__Number_divide__` | implemented |
| `__Number_floor__` | implemented |

* Independent of Steps 3 and 4, but placing it after Step 4 lets tests recover from errors with `__Cont_get__` instead of exiting.
* `__Number_toString__` comes first so that results can be printed.
* Every Number is a float. `__Number_fromString__` accepts only ASCII decimal notation (optional sign, fraction and exponent); anything else, a non-String argument, or a value that overflows goes to `onError`.
* `__Number_toString__` prints integer values below 1e21 without a decimal point, other finite values in shortest form, and `inf`, `-inf`, `nan` for non-finite results of arithmetic.

## Step 6: Ref

| Primitive | Status |
|---|---|
| `__Ref_new__` | implemented |
| `__Ref_get__` | implemented |
| `__Ref_set__` | implemented |

* The first mutable state. It does not need key lookup, so it is the simplest place to introduce mutation.
* `__Ref_get__` and `__Ref_set__` take no `onError`, so a non-Ref argument stops the VProc with an error.
* In traces a Ref is shown as `(__Ref__ "N")`, where N is its creation number, not its content, because a Ref may contain itself.

## Step 7: PDict

| Primitive | Status |
|---|---|
| `__PDict_put__` | implemented |
| `__PDict_lookup_or__` | implemented |

* Depends on Step 4: the `onMissing` example uses `__CChain_pop_LCont__`.
* Key equality (`values_equal` in `test/vproc_step.py`, shared with Dict in Step 8): Strings compare by content (String literals at different source positions are different AST nodes; see the 2026/4/24 entry in `memo.txt`), Numbers by value, null equals null, and any other value (Ref, Closure, LList, ...) only to itself. A String and a Number are never equal.
* `stdin`, `stdout`, `stderr` and `__exit__` in `include/util.se` look up the PDict.

## Step 8: MSeq & Dict

| Primitive | Status |
|---|---|
| `__MSeq_new__` | implemented |
| `__MSeq_is_empty__` | implemented |
| `__MSeq_append__` | implemented |
| `__MSeq_uncons__` | implemented |
| `__MSeq_set_head__` | implemented |
| `__Dict_new__` | implemented |
| `__Dict_put__` | implemented |
| `__Dict_lookup_or__` | implemented |
| `__Dict_remove__` | implemented |
| `__Dict_keys__` | implemented |

* Dict uses the same key equality as the PDict (Step 7).
* `__MSeq_uncons__` returns the tail as a new MSeq that does not share storage with the original.
* `__MSeq_set_head__` on an empty MSeq goes to `onError`, following its Behavior and Error entries (its Description says the head is created instead).
* Dict primitives take no `onError`, so a non-Dict argument stops the VProc with an error.
* `__Dict_keys__` returns the keys in insertion order. Replacing a value keeps the key's position; removing and re-adding a key moves it to the end.
* In traces an MSeq or Dict is shown as `(__MSeq__ "N")` or `(__Dict__ "N")`, like a Ref.
* `__Dict_keys__` returns a `Seq` (see "Sequence" in [SPEC.md](SPEC.html)).
* Both are needed by Step 11 (`commandMSeq`, `envDict`).

## Step 9: OS I/O streams

| Primitive | Status |
|---|---|
| `__OS_pipe__` | implemented |
| `__OS_write__` | implemented |
| `__OS_read__` | implemented |
| `__OS_close__` | implemented |
| `__OS_get_String__` | implemented |

* `__OS_get_String__` reads one line from standard input, including its newline, and passes null at EOF.
* Using a closed Stream goes to `onError` (`closed input Stream` / `closed output Stream`); the closed fd is never touched, because its number may already belong to a newer pipe.
* `__OS_read__` decodes UTF-8 across reads, so a character split between reads is returned whole by the later read; invalid bytes become U+FFFD.
* Strings produced at run time (read data, error messages, ...) are escaped as String literals are, so backslashes and quotes in the data are kept as they are.
* `__OS_read__` is non-blocking and returns `""` when no data is available. A reader that waits for data needs to yield to other VProcs, which is done in Step 10.

## Step 10: VProc

| Primitive | Status |
|---|---|
| `__VProc_current__` | implemented |
| `__VProc_suspend__` | implemented |
| `__VProc_resume__` | implemented |
| `__VProc_spawn__` | implemented |

* Round-robin scheduler (`Scheduler` in `test/vproc_step.py`); each scheduling step is one Operator application.
* `__VProc_spawn__ onError entryPoint sourceFileName errorSink argumentSeq inputSeq outputSink ,(VProc)`: `onError` and `entryPoint` were added to the signature. It runs `bin/includer` and `bin/resolver` (found relative to `test/vproc_step.py`; `include/` is the include path) on `sourceFileName` (relative to the current directory).
* Memory is shared between VProcs (TERMS.md was changed). Each closure keeps the GVEnv of the module it was defined in, so a closure passed from a parent still refers to the parent's globals when a child runs it.
* Termination and exit codes are described under "Runtime Model" in SPEC.md: a runtime error finishes only that VProc; `__OS_exit_*__` exits the interpreter; a deadlock exits with code 2.
* `__main__` was added to `include/util.se`. How the first VProc gets its err/args/in/out from the OS streams is still open; `test/vproc_run.py` runs a named function with no arguments as before.
* Blocking I/O (`__OS_get_String__`) stops every VProc while it waits; this is accepted for now.
* A runtime error now finishes only the VProc and leaves the exit code at 0, so the logs of the error cases of Steps 4, 5, 6 and 8 changed from `Error at step S: ... / exit: 1` to `Error in VProc 1 at step S: ... / exit: 0`.
* Tests (`test/step10`): `test.se` starts child VProcs from `child.se` (one entry point per case), `main.se` (`"__main__"`) and `bad.se` (a syntax error).

| Case | What it checks |
|---|---|
| case1 | `__VProc_current__` returns the same VProc each time |
| case2 | a child runs another program, gets arguments through a Seq, and writes through a Sink defined by the parent (per-closure GVEnv); Refs are shared |
| case3 | a VProc that suspends itself stops until another VProc resumes it, then continues with its LCont |
| case4 | suspending another ready VProc stops it until it is resumed; the no-op cases of suspend and resume |
| case5 | `onError` for a missing file, a syntax error, an undefined entry point and a non-String entry point |
| case6 | a runtime error in one child finishes only that child; another child keeps running; exit code 0 |
| case7 | `__OS_exit_error__` in a child exits the interpreter with code 1 while the parent is still running |
| case8 | `__OS_exit_normal__` in a child exits the interpreter with code 0 |
| case9 | deadlock: both VProcs blocked, exit code 2 |
| case10 | `"__main__"` puts err/in/out into the child's PDict and calls `main`; the parent's PDict is not affected |
| case11 | a non-VProc argument finishes the VProc with an error |

## Step 11: External processes

| Primitive | Status |
|---|---|
| `__OS_get_envDict__` | not implemented |
| `__OS_spawn__` | not implemented |
| `__OS_poll__` | not implemented |

* Depends on Dict (`envDict`) and MSeq (`commandMSeq`) from Step 8, and on streams from Step 9.
* `__OS_poll__` is non-blocking, so waiting for a process to exit uses the scheduler from Step 10.
