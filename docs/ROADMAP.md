# Primitive Implementation Roadmap

This document gives an order for implementing (or re-implementing) the built-in primitives defined in [PRIMITIVES.md](PRIMITIVES.html).
Each step depends only on the steps before it, so the primitives in a step can be tested with what is already available.
The test directory for a step is `test/stepN`. Existing step directories are `step1` to `step5` and `step9`.

Status shows whether `test/vproc_step.py` currently has the primitive (as of 2026-10-20).

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
| `__Ref_new__` | not implemented |
| `__Ref_get__` | not implemented |
| `__Ref_set__` | not implemented |

* The first mutable state. It does not need key lookup, so it is the simplest place to introduce mutation.

## Step 7: PDict

| Primitive | Status |
|---|---|
| `__PDict_put__` | not implemented |
| `__PDict_lookup_or__` | not implemented |

* Depends on Step 4: the `onMissing` example uses `__CChain_pop_LCont__`.
* Requires value equality for keys. Decide it before this step (see the 2026/4/24 entry in `memo.txt`: String literals from different source positions are different AST nodes and must be compared by content).
* `stdin`, `stdout`, `stderr` and `__exit__` in `include/util.se` look up the PDict.

## Step 8: MSeq & Dict

| Primitive | Status |
|---|---|
| `__MSeq_new__` | not implemented |
| `__MSeq_is_empty__` | not implemented |
| `__MSeq_append__` | not implemented |
| `__MSeq_uncons__` | not implemented |
| `__MSeq_set_head__` | not implemented |
| `__Dict_new__` | not implemented |
| `__Dict_put__` | not implemented |
| `__Dict_lookup_or__` | not implemented |
| `__Dict_remove__` | not implemented |
| `__Dict_keys__` | not implemented |

* Dict uses the same key equality as the PDict (Step 7).
* `__Dict_keys__` returns a `Seq` (see "Sequence" in [SPEC.md](SPEC.html)).
* Both are needed by Step 11 (`commandMSeq`, `envDict`).

## Step 9: OS I/O streams

| Primitive | Status |
|---|---|
| `__OS_pipe__` | implemented |
| `__OS_write__` | implemented |
| `__OS_read__` | implemented |
| `__OS_close__` | implemented |
| `__OS_get_String__` | not implemented |

* `__OS_read__` is non-blocking and returns `""` when no data is available. A reader that waits for data needs to yield to other VProcs, which is done in Step 10.

## Step 10: VProc

| Primitive | Status |
|---|---|
| `__VProc_current__` | not implemented |
| `__VProc_suspend__` | not implemented |
| `__VProc_resume__` | not implemented |
| `__VProc_spawn__` | not implemented |

* Needs a scheduler with a ready queue and a blocked queue.
* Implement `__VProc_spawn__` last: it needs the loader (includer and resolver), the `__main__` wrapper, Seq/Sink arguments and a fresh PDict (Step 7).

## Step 11: External processes

| Primitive | Status |
|---|---|
| `__OS_get_envDict__` | not implemented |
| `__OS_spawn__` | not implemented |
| `__OS_poll__` | not implemented |

* Depends on Dict (`envDict`) and MSeq (`commandMSeq`) from Step 8, and on streams from Step 9.
* `__OS_poll__` is non-blocking, so waiting for a process to exit uses the scheduler from Step 10.
