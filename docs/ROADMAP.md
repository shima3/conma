# Primitive Implementation Roadmap

This document gives an order for implementing (or re-implementing) the built-in primitives defined in [PRIMITIVES.md](PRIMITIVES.html).
Each step depends only on the steps before it, so the primitives in a step can be tested with what is already available.
The test directory for a step is `test/stepN`. Existing step directories are `step1` to `step34`.

Status shows whether `test/vproc_step.py` currently has the primitive (as of 2026-10-30).

## Tests

Each `test/stepN/test.sh` builds `test.se` with `test/resolver.sh`, runs each case function with `test/vproc_run.py` (`test/step13` also starts programs with `--main`), and compares the output with `caseN.log`. From `step2` on, `exit: <code>` is appended to the output; `step1` compares the full `--trace` output instead. No output means every case passed.

```
test/run_all.sh
CONMA_VM=rust test/run_all.sh
```

`test/run_all.sh` runs every `test.sh` under `test/` (the steps, the modules and the self-hosting tests) and prints `== name` before each. With `-j N` (`test/run_all.sh -j 8`), it runs N tests at a time and prints the output of each test, in the same order, when all have ended; `consh/test/run_all.sh` takes `-j N` too.

* Each `test.sh` sources `test/common.sh`, which moves to the test directory and makes a temporary directory `$tmp` (`mktemp -d`, removed when the test ends); `resolve test.se` builds `$tmp/test.module.se` with `test/resolver.sh`, and the logs of the cases are written in `$tmp`. So tests of different steps can run at the same time. The same test cannot run twice at the same time, since some tests make a `work/` directory or links in their own directory (`step14`, `step16`, `step23`, `step24`, `selfhost6`, `selfhost7`). (`step33` writes its file in `$tmp`, which it passes in `STEP33_DIR`.) The ConSh tests (`consh/test`) do the same with their own `consh/test/common.sh`, and `bin/consh` builds its module in `CONSH_MODULE_DIR` (`/tmp` by default) through a temporary file, so that ConSh started at the same time never reads a half-written module. Where an output holds a path in `$tmp` (the module file in the trace of `step1`, file names in `selfhost2` and `selfhost5`), it is written as `<tmp>`.
* `test/resolver.sh [path/]file.se [dir]` writes `dir/file.ast.se` and `dir/file.module.se` (`dir` is `/tmp` by default).

* `test.sh` starts the VM through `test/vm.sh`: the Python VM (`test/vproc_run.py`) by default, the Rust VM (`vm/`, see `vm/README.md`) when `CONMA_VM=rust`. The Python VM runs only concise ConMa, so `test/vm.sh` first translates a module of fast ConMa (with `(Uses "...")`) with `bin/concise` (Step 41).
* Standard output and standard error go to the same log, so `PYTHONUNBUFFERED=1` keeps their order.
* Standard input is empty (`/dev/null`) unless a case needs it (`test/step9` case5 reads `input.txt`; `test/step13` gives several cases an input file; `test/step20` case6 reads `input.txt`; `test/step21` gives each case its own input).
* Cases that write files (`test/step14`) write only in a `work/` directory that `test.sh` empties before each case and removes at the end. Absolute paths of the test directory are printed as `<dir>`, so that the logs do not depend on where the repository is.
* Cases that run several VProcs make their output independent of how the scheduler interleaves them: a VProc writes only after it has synchronized with the others through Refs and `__VProc_suspend__` / `__VProc_resume__`.
* `test/stress.sh [count] [dir ...]` runs the tests (all by default, with the Rust VM unless `CONMA_VM` says otherwise) with `CONMA_SEED` from 1 to `count` (10 by default), so that the scheduler runs the VProcs in a different order each time (see "Scheduling and Termination" in DESIGN.md), and prints the differences with the seed. Differences only in the step number of an error message (`at step N`), which counts the steps of every VProc, are not printed. The directories are relative to `test/`, so `../consh/test/step6` runs a ConSh test.

## Step 1: VM foundation

| Primitive | Status |
|---|---|
| `__NULL__` | implemented |
| `__is_null__` | implemented |
| `__OS_print__` | implemented |

* Goal: Check operator application, surplus arguments, partial application and LCont handling (see [DESIGN.md](DESIGN.html)).
* `__OS_print__` is needed first because every later test reports its result through standard output.
* Booleans and control flow (`__TRUE__`, `__FALSE__`, `__if_then_else__`, ...) are defined in `include/pure.se` on top of `__is_null__`, so they need no primitives (since Step 35, `__if_then_else__`, `__if__` and `__unless__` are primitives, with the ConMa versions in `include/control.se`; since 2026-11-13, `__TRUE__` and `__FALSE__` are defined in `include/ref/pure.se` for concise ConMa, and are primitives of fast ConMa, the feature `"boolean"`, and so are the control functions and `__Cont_get__`, defined in `include/ref/control.se` for concise ConMa, the feature `"control"`, docs/FAST-PLAN-ja.md 4.6).

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
* This must come before Step 4, because a CChain is traversed as an LList of CFrames (`CChain_print` in `test/step4`, the ConMa version of `__Cont_get__` in `include/control.se`).

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
* Library definitions that become available: `__Cont_get__` (escape continuation; since Step 34 a primitive, with the ConMa version in `include/control.se`), `__CChain_pop_LCont__`, `__noop__`.

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
* `__MSeq_set_head__` on an empty MSeq goes to `onError`, following its Behavior and Error entries. (Its Description used to say the head is created; PRIMITIVES.md was corrected to match.)
* Dict primitives take no `onError`, so a non-Dict argument stops the VProc with an error.
* `__Dict_keys__` returns the keys in insertion order. This is a detail of this implementation: PRIMITIVES.md leaves the order implementation-defined, and only the tests (`test/step8` case5) rely on it. Replacing a value keeps the key's position; removing and re-adding a key moves it to the end.
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
* `__OS_read__` checks for data with `select` instead of putting the pipe into non-blocking mode, because that mode would also apply to a child process that shares the pipe as its standard input (Step 11).

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
* `__main__` was added to `include/util.se`. How the first VProc gets its err/args/in/out from the OS streams was left open here and decided in Step 13.
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
| `__OS_get_envDict__` | implemented |
| `__OS_spawn__` | implemented |
| `__OS_poll__` | implemented |

* Depends on Dict (`envDict`) and MSeq (`commandMSeq`) from Step 8, and on streams from Step 9.
* `__OS_poll__` is non-blocking, so waiting for a process to exit uses the scheduler from Step 10 (`__OS_wait__` in Step 15 waits without busy waiting).
* Stream directions: `inputStream` is an input Stream (the read end, which the process reads), `outputStream` and `errorStream` are output Streams. null inherits the interpreter's own stream; the interpreter flushes its own output before starting the process.
* `__OS_spawn__` closes nothing; the program closes the Streams it no longer needs.
* The executable is looked up like `execvpe`, in the `PATH` of `envDict`; the environment is exactly `envDict`, and the working directory is the interpreter's.
* An exit caused by a signal is reported as the negative signal number. Processes still running when the interpreter exits are left running (checked by hand; not covered by a test).
* Tests (`test/step11`) use only `/bin/sh`, `/bin/echo`, `/bin/cat` and `/usr/bin/env`, and read output and exit codes only after `__OS_poll__` has seen the process exit. `test.sh` gives the interpreter an empty standard input, which case6 lets `cat` inherit.

| Case | What it checks |
|---|---|
| case1 | the standard output of a process through a pipe; the program closes its write end to get EOF |
| case2 | writing to a process's standard input through a pipe and reading its output |
| case3 | exit codes: `exit 3`, the same code from a second poll, `-9` for `kill -9`; standard error through a pipe |
| case4 | the environment is exactly `envDict`; a command name without `/` is found in its `PATH` |
| case5 | `__OS_get_envDict__` returns a copy; changing it does not change the interpreter's environment |
| case6 | null Streams inherit the interpreter's standard input and output, in order |
| case7 | `onError` for missing or non-executable commands, a bad command MSeq, wrong or closed Streams, a bad `envDict`, and polling a non-OSProc |
| case8 | other VProcs keep running while a VProc waits with `__OS_poll__` (`writer.se` feeds `cat`) |

## Step 12: String

| Primitive | Status |
|---|---|
| `__String_length__` | implemented |
| `__String_substring__` | implemented |
| `__String_concat__` | implemented |
| `__String_equal__` | implemented |

* Goal: Make it possible to take a command line apart in ConMa itself, as the first step toward a shell.
* Positions and lengths count code points (the same unit `__OS_read__` decodes to), and positions start at 0. Escape sequences count as one character.
* `__String_substring__` takes `onError` because reading past the end is a normal condition while scanning a String. A position that is not an integer, or `0 <= start <= end <= length` that does not hold, goes to `onError`.
* The other primitives take no `onError`. A non-String argument (or a non-Number position) stops the VProc with an error, as in the Number primitives.
* `__String_equal__` compares by content, like `values_equal`, so a String made at run time equals a literal with the same content. Before this step a String could only be compared through a Dict lookup.
* One character is taken with `__String_substring__ onError s i (i+1)`; no separate primitive is needed.

| Case | What it checks |
|---|---|
| case1 | `__String_length__`: empty String, escape sequences, Japanese text, a String made by `__Number_toString__` |
| case2 | `__String_substring__`: `end` is excluded, empty results, the whole String, Japanese text, escape sequences |
| case3 | `onError` for positions out of range, a negative position, `start > end` and a non-integer position |
| case4 | `__String_concat__`: empty Strings, quotes and backslashes are kept, the result can be concatenated again |
| case5 | `__String_equal__`: by content, with Strings made at run time |
| case6 | splitting a command line into words with these primitives only (spaces only, no quoting) |
| case7 | a non-String argument finishes the VProc with an error |

## Step 13: Standard Streams and program start

| Primitive | Status |
|---|---|
| `__OS_get_stdin__` | implemented |
| `__OS_get_stdout__` | implemented |
| `__OS_get_stderr__` | implemented |

* Goal: Start the first VProc the normal way, through `__main__` with err/args/in/out made from the interpreter's standard streams, and let a program read standard input without blocking every VProc.
* The three primitives return the Streams of fds 0, 1 and 2. They return the same Stream each time. They work with `__OS_read__`, `__OS_write__`, `__OS_close__` and `__OS_spawn__` like pipe ends.
* Closing one of them marks the Stream closed (later use goes to `onError`) but keeps the fd open, because the interpreter still prints to it.
* `__OS_write__` to standard output or standard error flushes what `__OS_print__` and `__OS_print_error__` have buffered first, so the order is kept.
* Library definitions in `include/util.se` (depend on Step 12):
  * `__Stream_sink__ stream`: a Sink that writes each value to an output Stream.
  * `__Stream_lines__ stream`: a Seq that reads an input Stream to EOF and passes each line, including its newline; a last line without a newline is passed as it is. While no data is available it waits with `__OS_wait_readable__` (Step 15; before that, it read again in a loop).
  * `__start__ args`: builds err, in and out from the three Streams and calls `__main__`.
* `test/vproc_run.py --main <module_file> [<arg> ...]` applies `__start__` to a Seq of the `arg` Strings. The Seq holds only the arguments, not the program name. Without `--main`, it runs a named function with no arguments as before.
* `__OS_get_String__` reads through Python's own buffer, so a program should not mix it with the stdin Stream.
* Tests (`test/step13`): case1 to case7 run functions of `test.se`, with `input.txt` or `lines.txt` as standard input where needed. case8 to case10 run `args.se` and `exit.se` with `--main`. Reading input that arrives later (`(sleep 1; echo x) | ...`) was checked by hand and is not covered by a test.

| Case | What it checks |
|---|---|
| case1 | writes to the stdout and stderr Streams stay in order with `__OS_print__` and `__OS_print_error__`; each call returns the same Stream |
| case2 | reading the stdin Stream with `__OS_read__` to EOF |
| case3 | `__Stream_lines__`: an empty line, Japanese text, a last line without a newline |
| case4 | `__Stream_sink__` with several values in one call; `__Stream_lines__` on empty input passes nothing |
| case5 | a closed stdout Stream goes to `onError`, also when it is got again; `__OS_print__` still works |
| case6 | `onError` for writing to the stdin Stream and reading from the stdout Stream |
| case7 | the standard Streams passed to `__OS_spawn__` (`/bin/cat` reads the interpreter's input) |
| case8 | `--main` with arguments: args, stdin lines through `stdin`, a message through `stderr`, `__print__` |
| case9 | `--main` with no arguments and empty input |
| case10 | `__exit__` leaves `main` early |

## Step 14: Files and the current directory

| Primitive | Status |
|---|---|
| `__OS_open__` | implemented |
| `__OS_chdir__` | implemented |
| `__OS_getcwd__` | implemented |

* Goal: Make redirection (`<`, `>`, `>>`) and the `cd` and `pwd` built-ins of a shell possible. Both only extend the Streams of Step 9 and the processes of Step 11.
* `__OS_open__ onError path mode ,(Stream)`: `"r"` gives an input Stream; `"w"` (create or empty) and `"a"` (create or append) give an output Stream. A file Stream works like a pipe end, so it can be the standard input or output of `__OS_spawn__`.
* Every error of `__OS_open__` and `__OS_chdir__` goes to `onError`, also a non-String argument, as in the other OS primitives: a missing file or directory, a directory opened as a file, permission denied, an invalid mode. `__OS_getcwd__` takes no `onError`.
* The current directory belonged to the interpreter, so `__OS_chdir__` changed it for every VProc (each VProc has its own since Step 23). It is used for relative paths in `__OS_open__`, `__OS_chdir__` and `__VProc_spawn__`, and as the working directory of `__OS_spawn__`.
* Tests (`test/step14`): `test.sh` gives every case a new empty `work/` directory to write in, and removes it at the end. The current directory is printed as `<dir>`, so that the logs do not depend on where the repository is.

| Case | What it checks |
|---|---|
| case1 | `"r"` gives an input Stream that reads to EOF; a closed file Stream goes to `onError` |
| case2 | `"w"` creates or empties, `"a"` appends, also to a new file; an output Stream cannot be read |
| case3 | `onError` for a missing file, a missing directory, a directory (`"r"` and `"w"`), an invalid mode, and non-String arguments |
| case4 | `__OS_getcwd__`; `__OS_chdir__` changes how relative paths are resolved; `onError` for a missing directory, a file and a non-String path |
| case5 | redirection: files as the standard input and output of `/bin/cat` |
| case6 | after `__OS_chdir__`, a child process (`/bin/sh -c pwd`) and `__VProc_spawn__` (`../child.se`) use the new directory |

## Step 15: Waiting for the OS

| Primitive | Status |
|---|---|
| `__OS_wait__` | implemented |
| `__OS_wait_readable__` | implemented |

* Goal: Stop busy waiting, so that a VProc can wait for a process or for input without using the CPU. Needed for background jobs (`&`, `wait`) in a shell.
* `__OS_wait__ onError OSProc ,(status)` waits for a process to exit and returns the exit code as `__OS_poll__` does. `__OS_wait_readable__ onError inputStream ,()` waits until the next `__OS_read__` will not return `""`. Both return at once when the event has already happened. `__OS_poll__` is kept.
* A waiting VProc is in the new state `waiting`. Other VProcs keep running. While a ready VProc exists, the scheduler checks the waiting ones before each step without blocking. Otherwise it sleeps in `select` until a Stream is readable or SIGCHLD arrives (through `signal.set_wakeup_fd`).
* VProcs that wait are not a deadlock, even if all the others are blocked. `__VProc_suspend__` and `__VProc_resume__` do not change a waiting VProc.
* A Stream closed by another VProc wakes the VProc waiting on it; the next `__OS_read__` goes to `onError`.
* `__Stream_lines__` (Step 13) now waits with `__OS_wait_readable__` instead of reading again in a loop.
* Tests (`test/step15`): case2 and case3 run with `--trace` for a delay of 0.2 and 1 second, and compare the number of steps (the `[done]` line). The numbers are the same because waiting uses no steps. With busy waiting they would grow with the delay. case4 depends on timing: the worker needs much less than 0.5 seconds.

| Case | What it checks |
|---|---|
| case1 | `__OS_wait__` returns the exit code, the same code again for an exited process, `-9` for a signal; a child's output is inherited |
| case2 | waiting for `sleep` uses the same number of steps for 0.2 and 1 second |
| case3 | `__OS_wait_readable__` on a pipe waits for data, then for EOF; same number of steps for both delays |
| case4 | another VProc keeps running while one waits |
| case5 | returns at once when data or EOF is there; `onError` for a closed Stream, an output Stream and a non-OSProc |
| case6 | a VProc that waits is not a deadlock while its parent is suspended; it resumes the parent afterwards |
| case7 | closing a Stream in another VProc wakes the VProc waiting on it, and its read goes to `onError` |

## Step 16: Type names and real paths

| Primitive | Status |
|---|---|
| `__type_name__` | implemented |
| `__OS_realpath__` | implemented |

* Goal: Provide what the ConMa versions of the front end tools need (see [SELFHOST.md](SELFHOST.html)). `__AST_toSexp__` uses `__type_name__` to tell Strings, Numbers and AST nodes apart, and `includer` uses `__OS_realpath__` to check that a file exists and to find duplicate files.
* `__type_name__ Value ,(String)` returns `"null"` (also for `__NULL__`, an empty LList and an empty CChain), `"String"` (a literal or one made at run time), `"Number"`, `"Closure"` (also for a primitive function), `"Ref"`, `"CFrame"`, `"LList"`, `"MSeq"`, `"Dict"`, `"VProc"`, `"OSProc"` or `"Stream"` (the same for input and output Streams). A Boolean, a Seq and a Sink are Closures, and a CChain is an LList. It takes no `onError`, because every value has a type.
* `__OS_realpath__ onError path ,(String)` returns the absolute path with `..`, `.` and symbolic links resolved. A relative path is relative to the current directory (Step 14). A path that does not exist, or a non-String argument, goes to `onError`. Python 3.9 has no `strict` mode for `os.path.realpath`, so the path is checked with `os.stat` first; a path through a missing directory (`none/..`) is therefore an error, as in the OS.
* `value_type_name` in `test/vproc_step.py` gives type names for error messages, but it returns `"input Stream"` / `"output Stream"` and Python class names for other values, so `__type_name__` needs its own fixed list of names.
* Tests (`test/step16`): `test.sh` gives every case a new `work/` directory with `sub/` and a symbolic link `up` to the test directory, and removes it at the end. The test directory is printed as `<dir>`.

| Case | What it checks |
|---|---|
| case1 | `__type_name__`: `"null"` for `__NULL__` and an empty LList; `"String"` for literals, the empty String and Strings made at run time; `"Number"` |
| case2 | `"Closure"` for a FuncExp, a defined function, a primitive, `__TRUE__`, a Boolean result, a Seq and a Sink |
| case3 | `"Ref"`, `"CFrame"`, `"LList"`, `"MSeq"`, `"Dict"`, `"VProc"`; an empty CChain gives `"null"`, a CChain with a CFrame gives `"LList"` |
| case4 | `"Stream"` for stdin, stdout, both pipe ends and a file Stream, also after closing it; `"OSProc"` |
| case5 | `__OS_realpath__`: an absolute path; relative paths before and after `__OS_chdir__` |
| case6 | paths with `.`, `..` and the symbolic link are normalized, also an absolute path |
| case7 | `onError` for a missing path, a path through a missing directory, a path under a file, the empty String and a non-String argument |

## Step 17: Waiting for a VProc

| Primitive | Status |
|---|---|
| `__VProc_wait__` | implemented |

* Goal: Let a VProc call a function of another program and get its result, with `__VProc_spawn__` instead of an `eval` primitive. The parent spawns the child, waits for it to finish, and reads the result that the child passed to a Sink of the parent (see case9).
* `__VProc_wait__ onError VProc ,()` waits until the VProc has finished, normally or with a runtime error, and returns at once if it already has. A non-VProc argument or the current VProc goes to `onError`. The result of the VProc is not returned: an empty Ref after waiting shows that the child finished without a result.
* A VProc that waits for a VProc is in the state `waiting`, as for the OS waits of Step 15, and `__VProc_suspend__` and `__VProc_resume__` do not change it. The scheduler wakes it at the start of a scheduling step after its VProc has finished.
* Unlike a VProc waiting for the OS, a VProc waiting for a VProc counts as blocked for the deadlock check: if no VProc is ready and none waits for the OS, it can never wake up. The deadlock message lists it with the blocked VProcs.
* Waiting with a Ref and `__VProc_suspend__` / `__VProc_resume__` loses the wakeup if the child resumes the parent before the parent has suspended itself, and leaves the parent blocked if the child stops with a runtime error; `__VProc_wait__` avoids both.
* case9 sets the CChain back with `__CChain_get__` and `__CChain_set__` to return the value of `onError`, as `__Parser_main__` does. When this Step was made, the `return` of `__Cont_get__` could not pass values to the caller; Step 18 fixed this.
* Tests (`test/step17`): `test.se` starts child VProcs from `child.se` (one entry point per case). The parent writes only after `__VProc_wait__` returns, so the output does not depend on how the scheduler interleaves the VProcs.

| Case | What it checks |
|---|---|
| case1 | the parent waits for a child that runs for a while, then reads the value the child passed to its Sink |
| case2 | waiting for a VProc that has already finished returns at once, also a second time |
| case3 | a child that stops with a runtime error wakes the parent; the Ref is still null |
| case4 | `onError` for a String, a Number and the current VProc |
| case5 | two VProcs wait for the same VProc, and the parent waits for both |
| case6 | a child suspends and resumes the parent that waits for it; the parent still waits until the child has finished |
| case7 | deadlock: the child suspends itself and the parent waits for it, exit code 2 |
| case8 | deadlock: the parent and the child wait for each other, exit code 2 |
| case9 | `call`, made of `__VProc_spawn__`, `__VProc_wait__` and a Ref, calls `add` in `child.se` and returns its result; a runtime error and an undefined entry point give `onError` |

## Step 18: Variadic functions and `__Cont_get__`

No new primitive; the language and `__Cont_get__` (then in `include/util.se`, since Step 34 in `include/control.se`) change.

* Goal: Let the `return` of `__Cont_get__` pass values to the caller, as the example in SPEC.md (`return "12" "34"`) intends. Before this Step, `return` was a closure with an empty Head, so its arguments were surplus operands: they were pushed onto the CChain as a Seq Closure and lost when `return` set the CChain back. The definition in SPEC.md and the one in `util.se` differed, and neither passed values. A new kind of continuation value would also fix this, but variadic functions are simpler as a concept and useful elsewhere.
* Syntax: `Head = "(", { Parameter }, ")" | Parameter`. A Function `, args Body` is variadic: an application binds a Seq of all its operands to `args` (an empty Seq without operands). It is always executed, even without operands, and takes the operands of one application only, so there are never surplus operands. See "Variadic Functions" in SPEC.md.
* AST: a variadic Head is `(Head (line col) Rest (Variable (line col) "args"))` at the position of the parameter. `bin/parser`, `bin/resolver`, `include/parser.se` and `include/resolver.se` handle it; `test/selfhost5` and `test/selfhost7` compare a source with variadic Heads.
* VM (`test/vproc_step.py`): a Closure has a `variadic` flag, set from a Head with `Rest`. Applying a variadic Closure pushes the LCont, binds a SeqClosure of the OList to the parameter, and runs the Body. Traces write the Head with `Rest` and read it back.
* `__Cont_get__` gives a variadic `return`: it sets the CChain back to that of the caller, pops the caller's LCont and applies the Seq of its arguments to it, so the LCont gets all the values (none without arguments). If there is no caller (the CChain is empty), `return` empties the CChain and the VProc finishes, as before (for example `exit` in `__main__`).
* Library definitions in `include/util.se` that use it:
  * `__Seq_get__ onEmpty Seq ,(first rest)` (from SPEC.md). The definition in SPEC.md took as `rest`, after the last element, the LCont that calls `onEmpty`, so a later `__Seq_get__` on that `rest` applied the first `onEmpty`, not its own. Now that LCont applies `onEmpty` only for an empty Seq.
  * `rest` is not only the top CFrame of the CChain. When the Sink gets an element, the rest of the Seq is every CFrame above the LCont of the application of the Seq: the Seq Closure of surplus operands, the LCont of the application of the Sink, and the LConts of functions that the Seq called with an LCont (a Seq made of other Seqs, such as `seqABC` in case10). The first version took only the top CFrame, so `return` dropped the others and the elements they would pass were lost. `rest` now keeps those CFrames (counted with `__LList_length__` of the CChain before and inside the Sink) and, when applied to a Sink, puts them back on top of the CChain of its caller and returns the Sink to them. After the last element there are no such CFrames, so `rest` returns the Sink at once and is an empty Seq. `rest` can be applied more than once, and passes the same elements each time.
  * `__Seq_print__ Seq` writes the elements with `__OS_print__`, and `__println__ , args` writes all its arguments and a newline. They have reserved names, so that they do not clash with the `println` that many tests define for themselves.
* `test/step4` case9 and case10 check that the `return` of `__Cont_get__` passes values and returns without them.
* Tests (`test/step18`):

| Case | What it checks |
|---|---|
| case1 | a variadic function with 0, 1 and 3 operands; it runs without operands; `__type_name__` of the function and of the Seq |
| case2 | only the operands of one application are in the Seq: `a3 a4` applied by the LCont are not |
| case3 | a variadic LCont gets all the values returned, also none |
| case4 | in the Operator position it runs even without operands; it is passed without running as an operand or with `__NULL__` |
| case5 | the variadic parameter is one variable for de Bruijn indexes, also in nested functions |
| case6 | `return` of `__Cont_get__` with two values, with none, and from a nested function inside the function |
| case7 | `return` in the first function of a VProc (no caller) finishes the VProc |
| case8 | `__Seq_get__` four times on a Seq that passes its elements together, one by one, with one element, with none, and on a variadic Seq; the rest after the last element is empty, and each call applies its own `onEmpty` |
| case9 | `__println__` with no, one and three arguments, and `__Seq_print__`; a file that defines its own `println` and `printSeq` does not affect them |
| case10 | `__Seq_get__` and `__Seq_print__` on Seqs that call another Seq with an LCont and pass more elements afterwards; a `rest` applied twice |

## Step 19: Character codes

| Primitive | Status |
|---|---|
| `__String_toCode__` | implemented |
| `__String_fromCode__` | implemented |

* Goal: Let a program look at the code point of a character, so that it can classify characters by range and write characters that a String literal cannot (a String literal has only the escapes `\"`, `\\`, `\n`, `\t`, `\r`). The first use is `__String_repr__` in `include/util.se`, which the ConMa front end uses to write values in error messages as Python's `repr` does (see [SELFHOST.md](SELFHOST.html)). A lexer for a shell can also use it.
* `__String_toCode__ onError String ,(Number)` returns the code point of a String of one character; a String of another length goes to `onError`. `__String_fromCode__ onError Number ,(String)` returns the String of one character; a Number that is not an integer from 0 to 0x10FFFF, or is a surrogate (0xD800 to 0xDFFF), goes to `onError`. As for the other String primitives, a non-String or non-Number argument stops the VProc with an error.
* `__String_repr__ String ,(String)` quotes the String as Python's `repr` does: in single quotes, or in double quotes if it has `'` and no `"`; `\`, the quote, newline, tab and carriage return are escaped, and characters that Python does not print are written as `\xNN`, `\uNNNN` or `\UNNNNNNNN`. A character is not printed if it is in the categories Cc, Cf, Zs, Zl, Zp or Co (except the space), as `str.isprintable` of Python decides; the ranges of these characters are written in `util.se` for Unicode 15.1 (Python 3.13). Unassigned characters (category Cn, such as U+FFFF) are written as they are, whereas Python escapes them. ASCII characters from 32 to 126 are printed without looking at the table of ranges, because most characters are of this kind and the lookup takes many steps.
* Tests (`test/step19`):

| Case | What it checks |
|---|---|
| case1 | `__String_toCode__`: ASCII, escape sequences, Japanese text, a character outside the BMP |
| case2 | `__String_fromCode__`: the inverse of `__String_toCode__`; the result equals a String literal; U+0000 is a String of length 1 |
| case3 | `onError` for a String of length 0 and 2, a non-integer, a negative Number, 0x110000 and the two ends of the surrogates; 0x10FFFF is accepted |
| case4, case5 | a non-String and a non-Number argument stop the VProc with an error |
| case6 | `__String_repr__`: the choice of quotes; escapes of `\`, the quote, newline, tab and carriage return; Japanese text is not escaped |
| case7 | `__String_repr__` of the characters at the ends of the ranges that are not printed, and next to them; an unassigned character is not escaped |

## Step 20: LazyList

| Primitive | Status |
|---|---|
| `__LazyList_claim__` | implemented |
| `__LazyList_hold__` | implemented |
| `__LazyList_publish__` | implemented |
| `__Ref_wait_change__` | implemented |

* Goal: Give a list that can be taken apart from the front cheaply and walked more than once. A shell on ConMa (ConSh) passes standard input, arguments and outputs of commands as lists that are split as `(a b . rest)` and kept in variables. With a `Seq`, each element needs `__Seq_get__`, which counts and moves CFrames of the CChain, and a `rest` applied again runs the rest of the `Seq` again, so a `Seq` of the lines of standard input would read it again. A `LazyList` evaluates each element once and keeps it.
* The `LList` itself is not made lazy: the CChain is an `LList`, `__LList_uncons__` is a primitive that cannot apply a thunk, and the library uses `LList` as a strict list. A `LazyList` is a `Ref` whose value is a thunk, null (empty) or an `LList` cell whose tail is a `LazyList`, so an evaluated cell is an ordinary `LList` cell. See "LazyList" in SPEC.md.
* Functions: `__LazyList_new__`, `__LazyList_empty__`, `__LazyList_cons__`, `__LazyList_force__`, `__LazyList_uncons__`, `__LazyList_is_empty__`, `__LazyList_fromSeq__`, `__LazyList_toSeq__`, `__LazyList_pipe__`.
* Pipe: `__LazyList_pipe__ capacity onClosed` returns a `LazyList`, a `Sink`, `close` and `cancel`, so that the values written to the `Sink` in one VProc reach another VProc as the elements of a `LazyList` (for `f | g` in ConSh). The writing VProc holds the tail that has no value yet (`__LazyList_hold__`), publishes `(value . next)` for each value and null at `close`, and a VProc that evaluates the tail waits in `__LazyList_claim__` until it is published. If the writing VProc finishes without `close`, the reader gets `onError`. Unlike a rendezvous of two VProcs with `__VProc_suspend__` and `__VProc_resume__`, no wakeup can be lost.
* Pipe capacity: the writer may be at most `capacity` elements ahead of the readers, so a writer that never stops (`yes | head -1`) does not fill the memory. Each element is behind a `LazyList` with a thunk that, when a reader evaluates it, sets a state `Ref` to a new value and returns the held tail. The writer keeps a queue of these `LazyList`s not yet evaluated, and waits with `__Ref_wait_change__` on the state `Ref` while the queue is longer than `capacity`. It reads the state before testing the queue, so a reader that asks in between makes `__Ref_wait_change__` return at once. A counter of the elements asked for would need a read and a set by each reader, and two readers could make it go back.
* Pipe cancel: a reader applies `cancel` when it needs no more elements; the writer's next (or waiting) `Sink` application ends the `LazyList` and applies `onClosed` instead (SIGPIPE). The writer cannot tell that a reader has finished without `cancel`.
* `__Ref_wait_change__ Ref value` waits until the value of the `Ref` is not `value`. Polling with `__VProc_suspend__` and `__VProc_resume__` could lose a wakeup between testing the value and suspending; passing the value read before the test closes that gap.
* Primitives: the mark that a thunk is being evaluated is read and set by `__LazyList_claim__` and `__LazyList_publish__`, each in one Operator application, so that several VProcs may evaluate the same `LazyList`. With `__Ref_get__` and `__Ref_set__`, the scheduler could switch VProcs between reading the value and setting the mark, and the thunk could run twice. A general compare-and-set of a `Ref` would prevent that, but a VProc that finds the mark could then only wait by polling: adding itself to a list of waiters and `__VProc_suspend__` are two applications, and a `__VProc_resume__` between them would be lost. `__LazyList_claim__` makes the VProc wait in the scheduler (as `__VProc_wait__` does) and runs again when the mark is gone or its VProc has finished.
* The mark is the evaluating VProc, so `__LazyList_claim__` tells a `LazyList` that depends on itself (the mark is the current VProc) from one that another VProc is evaluating, and gives `onError` when the evaluating VProc has finished without setting the cell (for example, after a runtime error in the thunk).
* Leaving the evaluation: a thunk may leave `__LazyList_force__` by a continuation (an `onError`, or the `return` of a function outside it). Then the mark would stay: the same VProc would be told that the `LazyList` depends on itself, and another VProc would wait until the first one finished, which a long-running VProc (a shell) never does. So `__LazyList_claim__` also keeps the CChain at the claim. While the thunk runs, that CChain is the bottom of the VProc's CChain (identical `LList` cells, since a CChain is only consed onto); when it is not, the VProc has left the evaluation, and the `LazyList` is claimed again with its thunk. `__LazyList_force__` claims inside `__LazyList_force_claim__`, called with an LCont, so that the CChain at the claim is not empty (an empty CChain is the bottom of every CChain). A held `LazyList` (`__LazyList_hold__`) has no thunk and is never left; it waits for its VProc to publish it.
* VM (`test/vproc_step.py`): `Scheduler.wait_lazy` puts a VProc in `lazy` without returning from the primitive, so that it runs `__LazyList_claim__` again when it wakes; `check_lazy` wakes it at the next scheduling step after the value of the `Ref` is no longer a VProc that has not finished and has not left the evaluation (`lazy_abandoned` walks the CChain of that VProc down to the CChain at the claim). The `Ref` keeps the CChain and the thunk in `lazy_chain` and `lazy_thunk` (`HELD` for `__LazyList_hold__`). `wait_change` and `check_changes` do the same for `__Ref_wait_change__`, which returns before waiting. A VProc waiting for a `LazyList` or a `Ref` is counted as blocked for deadlock detection.
* Tests (`test/step20`):

| Case | What it checks |
|---|---|
| case1 | `__LazyList_empty__`, `__LazyList_cons__`, `__LazyList_is_empty__` and `__LazyList_uncons__`; `onEmpty` gets the continuation of `__LazyList_uncons__`; walking twice gives the same elements; `__type_name__` is `"Ref"` |
| case2 | a thunk runs only when its `LazyList` is first evaluated, and only once |
| case3 | an infinite `LazyList` is evaluated only as far as it is walked; a `LazyList` whose tail is itself |
| case4 | a thunk that returns a `LazyList` not yet evaluated; `append` returns `ys` itself when `xs` is empty |
| case5 | `__LazyList_fromSeq__` gets the elements of a `Seq` one at a time, for a `Seq` that passes elements together, one by one, none, and a variadic `Seq`; the `Seq` of `__LazyList_toSeq__` can be applied twice |
| case6 | a `LazyList` of the lines of standard input, walked twice, reads standard input once |
| case7 | a `LazyList` that evaluates itself while it is evaluated stops the interpreter with an error, exit code 1 |
| case8 | two VProcs (from `child.se`) walk the same `LazyList`; the one that comes second waits for the first; each thunk runs once, and both get all the elements |
| case9 | `__LazyList_claim__` returns the thunk and sets the mark; a second claim after the first `tryClaim` has returned (the evaluation was left) returns the thunk again; after `__LazyList_publish__` it returns the cell; an empty `LazyList`; a non-Ref and a Ref of a String give `onError` |
| case10 | the evaluating VProc finishes with a runtime error in the thunk; the VProc waiting for it and a later claim get `onError` |
| case11 | two VProcs evaluate `LazyList`s that depend on each other: deadlock, exit code 2 |
| case12, case13 | `__LazyList_publish__` of a `LazyList` that the VProc has not claimed, and of a cell that is not null or an `LList`, stop the VProc with an error |
| case14 | `__LazyList_pipe__` (capacity 3): the values written to the `Sink`, one or several at a time, become the elements in order; the elements written so far can be walked before `close`; walking twice gives the same elements |
| case15 | writing to the `Sink` after `close` stops the VProc with an error (`__LazyList_publish__` of the null tail) |
| case16 | a child VProc walks the `LazyList` of a pipe while the parent writes to it slowly; it waits for each value and gets all of them |
| case17 | a child VProc makes a pipe, writes one value and finishes without `close`; the VProc waiting for the tail (the held `LazyList` that the thunk of the next element returns) gets `onError` |
| case18 | a thunk that leaves the evaluation by an outer continuation the first time: the same VProc evaluates the `LazyList` again and the thunk runs again (no "depends on itself"); after that it is memoized |
| case19 | the same, while a child VProc waits for the `LazyList`: the child wakes up when the parent leaves, runs the thunk and gets the element |
| case20 | a pipe of capacity 1 read slowly by a child VProc: `wrote n` comes only after `read n-2` |
| case21 | a child VProc reads two elements and applies `cancel`: the writer's next `Sink` ends the `LazyList` and applies `onClosed`; the `LazyList` keeps the elements written before |
| case22 | writing past the capacity with no reader: the writer waits in `__Ref_wait_change__`, which is a deadlock (exit code 2) |
| case23 | `__Ref_wait_change__` returns at once when the value is already another one, and otherwise waits until a child VProc sets the `Ref` |

## Step 21: Standard input as a LazyList

No new primitive.

* Goal: Make the input of a program a `LazyList` of lines instead of a `Seq`, so that a program (and ConSh) can take lines from the front with no `__Seq_get__`, and a part of the input can be given to a child VProc without reading standard input again.
* `include/util.se`: `__LazyList_lines__ Stream` reads the Stream directly into a `LazyList` of lines; `__start__` passes it to `__main__` as `in`.
* Read position: `__main__` puts a `Ref` whose value is the `LazyList` of the lines not yet read in `PDict` under `"__STD_IN__"`, and goes on in `__main_run__`, which does not take `in`. If `PDict` or a continuation kept the first cell, all the lines read so far would stay in memory, which a `Seq` did not do. A Closure keeps every variable bound where it is made (its LVEnv), so `__start__` makes the err and out Sinks with `__Stream_newSink__` instead of `(,(value) ...)` after `in` is bound: these Sinks are kept in `PDict` and would keep the first cell through their LVEnv. `stdin` returns the `LazyList` at the read position without moving it; `__stdin_read__` returns the next line and moves it.
* `test/step13/args.se` reads standard input with `__stdin_read__` (its output is unchanged).
* Tests (`test/step21`, programs started with `--main`):

| Case | What it checks |
|---|---|
| case1 | `__stdin_read__` reads lines in order, the last one without a newline, then applies `onEmpty`; `stdin` shows the lines not yet read, twice, without moving the read position |
| case2 | empty standard input |
| case3 | a child started with `"__main__"` and the parent's `stdin` as its inputSeq reads the rest of the lines; the parent then reads the same lines |
| case4 | an endless standard input (`yes`): reading three lines reads only what it needs, and the program ends |
| case5 | `test/step21/memcheck.py` runs the program and counts, every 2000 steps after a garbage collection, the interpreter's `LList` cells that hold a line; reading 200 lines with `__stdin_read__` keeps at most 1 alive |
| case6 | the same, with the first `stdin` kept in `PDict`: almost all 200 stay alive, which shows that `memcheck.py` finds them |

## Step 22: Exit code

| Primitive | Status |
|---|---|
| `__OS_exit__` | implemented |

* Goal: Let a program exit with any exit code, not only 0 (`__OS_exit_normal__`) and 1 (`__OS_exit_error__`). ConSh exits with the status of its last command (`$?`), as a shell does.
* `__OS_exit__ onError Number ,()` exits the interpreter with the exit code `Number`, even if other VProcs are still running. A Number that is not an integer from 0 to 255 goes to `onError`; a non-Number argument stops the VProc with an error.
* Tests (`test/step22`):

| Case | What it checks |
|---|---|
| case1 | exit code 0; the output written before is not lost |
| case2 | exit code 3, while another VProc is still running |
| case3 | exit code 255 |
| case4 | `onError` for 256, -1 and 1.5, which do not exit |
| case5 | a non-Number argument stops the VProc with an error |

## Step 23: A current directory for each VProc

No new primitive.

* Goal: Let each VProc have its own current directory, so that a VProc that changes its directory does not move the others. ConSh runs each element of a pipeline, `&` and `$( )` in a VProc, and `cd` there must not change the directory of the shell (see `consh/PLAN-ja.md`).
* Each VProc keeps its current directory as an absolute path. The first VProc starts in the directory where the interpreter was started, and a VProc made by `__VProc_spawn__` starts in the current directory of the VProc that made it. `__OS_chdir__` changes only the current VProc's, to the real path of the directory (symbolic links resolved), as the OS gives after `chdir`. The interpreter itself never changes its own directory.
* The current directory of the current VProc is used for relative paths in `__OS_open__`, `__OS_chdir__`, `__OS_realpath__` and the `sourceFileName` of `__VProc_spawn__`, and as the working directory of `__OS_spawn__`; `__OS_getcwd__` returns it. The empty path is not joined to it, so that it stays an error.
* `test/vproc_step.py` keeps the directory outside the S-expression of a VProc, so a VProc read from an S-expression (`vproc_step.sh`) starts in the interpreter's directory.
* Tests (`test/step23`): `test.sh` gives every case a new `work/` directory with a subdirectory `a` and a symbolic link `link` to it. The current directory is printed as `<dir>`.

| Case | What it checks |
|---|---|
| case1 | a child VProc starts in the directory of the VProc that spawned it (`../test.se` is found from there); its `__OS_chdir__` does not move the parent |
| case2 | after `__OS_chdir__`, a relative path in `__OS_open__` and `__OS_realpath__`, and the working directory of a child process (`/bin/sh -c pwd`), are those of the VProc |
| case3 | `onError` for a missing directory, a file and the empty path; the directory does not change |
| case4 | through a symbolic link, the current directory is the real path |

## Step 24: Starting a closure in a new VProc

| Primitive | Status |
|---|---|
| `__VProc_start__` | implemented |

* Goal: Let a program run a closure of its own in a new VProc. ConSh runs each element of a pipeline, `&` and `$( )` in a VProc (see `consh/PLAN-ja.md`); `__VProc_spawn__` runs `bin/includer` and `bin/resolver` on a file each time, which takes about 0.3 seconds even for a small program.
* `__VProc_start__ Closure ,(VProc)` makes a VProc that applies `Closure` to no arguments, in the current directory of the current VProc, and adds it at the end of the ready queue. The closure keeps the GVEnv of its program, and the values it refers to are shared.
* Tests (`test/step24`): `test.sh` gives every case a new `work/` directory with a subdirectory `a`. The current directory is printed as `<dir>`.

| Case | What it checks |
|---|---|
| case1 | the child uses a local variable and a Ref of the closure and a global function; the parent waits with `__VProc_wait__` and reads the Ref |
| case2 | the child starts in the directory of the parent; its `__OS_chdir__` does not move the parent |
| case3 | a runtime error in the child finishes only the child |
| case4 | a non-Closure argument stops the calling VProc with an error |
| case5 | two children write their own Refs; the parent waits for both |

## Step 25: Waiting until an output Stream is writable

| Primitive | Status |
|---|---|
| `__OS_wait_writable__` | implemented |

* Goal: Let a VProc write to a pipe without blocking the interpreter. ConSh writes the elements of a LazyList to the standard input of an external command in one VProc, while another VProc reads the output of that command (see `consh/PLAN-ja.md`). `__OS_write__` blocks the whole interpreter when the pipe is full, so the command can stop on its full output pipe while nobody reads it.
* `__OS_wait_writable__ onError outputStream ,()` waits, like `__OS_wait_readable__`, until `select()` reports the Stream writable: for a pipe, until it has room for `PIPE_BUF` bytes, or its reader has closed it. A write of at most 512 bytes after it does not block.
* The scheduler (`check_waiting`) gives `select()` the input Streams that VProcs wait for as the read list and the output Streams as the write list.
* Tests (`test/step25`):

| Case | What it checks |
|---|---|
| case1 | one VProc writes 400 pieces of 512 bytes to a pipe, waiting before each, while another VProc reads them; more than the pipe buffer passes without blocking the interpreter |
| case2 | on a pipe whose reader is closed, it returns at once and the next `__OS_write__` gives `onError` |
| case3 | `onError` for an input Stream, a closed Stream and a non-Stream |

## Step 26: VProc numbers, stopping VProcs and signals

| Primitive | Status |
|---|---|
| `__VProc_id__` | implemented |
| `__VProc_kill__` | implemented |
| `__OS_getpid__` | implemented |
| `__OS_kill__` | implemented |

* Goal: Let ConSh give `$!` (the number of the VProc that `&` started), `wait` and `kill` for such VProcs, and `$$` (the process ID of the shell); see `consh/PLAN-ja.md`.
* `__VProc_kill__ VProc ,()` finishes the VProc at once (`Scheduler.kill` removes it from the ready queue and from every list of waiting VProcs). The VProcs waiting for it wake up as if it had finished by itself; a LazyList that it holds gives `onError` to the VProc that claims it. OS processes it started are not stopped; `__OS_kill__ onError OSProc signal ,()` sends them a signal.
* `__VProc_id__ VProc ,(Number)` gives the serial number of the VProc, and `__OS_getpid__ ,(Number)` the process ID of the interpreter.
* Tests (`test/step26`):

| Case | What it checks |
|---|---|
| case1 | the first VProc is 1 and the next one 2 |
| case2 | a VProc waiting for a Ref is stopped; the VProc waiting for it wakes up; stopping it again does nothing |
| case3 | a VProc that holds a LazyList is stopped; the VProc that claims the LazyList gets `onError` |
| case4 | a VProc stops itself |
| case5 | the process ID is positive; SIGTERM stops `sleep` (exit code -15); `onError` for signal 0 and a non-OSProc |

## Step 27: System configuration values

| Primitive | Status |
|---|---|
| `__OS_sysconf__` | implemented |

* Goal: Let ConSh limit the bytes of the arguments of an external command by the OS value of `ARG_MAX` when the environment variable `ARG_MAX` is not set, so that expanding an infinite LazyList into arguments stops with an error; see `consh/PLAN-ja.md`.
* `__OS_sysconf__ onError name ,(Number)` gives the value of the system configuration variable `name` (a String with one of the names of `sysconf(3)`, such as `"SC_ARG_MAX"`), from Python's `os.sysconf`.
* Tests (`test/step27`):

| Case | What it checks |
|---|---|
| case1 | `SC_ARG_MAX` and `SC_OPEN_MAX` are positive |
| case2 | `onError` for an unknown name |
| case3 | `onError` for a name that is not a String |

## Step 28: Searching in Strings

| Primitive | Status |
|---|---|
| `__String_find__` | implemented |

* Goal: Read long lines fast. `__LazyList_lines__` and `__Stream_lines__` in `include/util.se` looked for a newline one character at a time, and each `__String_length__` and `__String_substring__` call takes time in proportion to the length of the String, so the time grew faster than the length of a line (a line of 40000 characters took about 60 seconds in ConSh).
* `__String_find__ onError String pattern start ,(index)` gives the first position of `pattern` at or after `start`, or null. The line readers now find each newline with one call. In ConSh, reading a line of 40000 characters with `$( )` now takes no measurable time beyond the start-up.
* Tests (`test/step28`):

| Case | What it checks |
|---|---|
| case1 | the first position at or after `start`; null when there is none; an empty pattern is found at `start` |
| case2 | positions count characters, not bytes; a newline can be found |
| case3 | `onError` for a `start` out of range and for a non-integer `start` |

## Step 29: Directory listings

| Primitive | Status |
|---|---|
| `__OS_listdir__` | implemented |

* Goal: Let ConSh expand glob patterns such as `*.txt` (see `consh/PLAN-ja.md`).
* `__OS_listdir__ onError path ,(names)` gives the names in a directory, without `"."` and `".."`, sorted by code point, relative to the current directory of the VProc.
* Tests (`test/step29`, with the directory `test/step29/dir`):

| Case | What it checks |
|---|---|
| case1 | the names in code point order, with a name that starts with `.` and without `.` and `..` |
| case2 | a relative path is relative to the current directory of the VProc |
| case3 | `onError` for a missing path and for a path that is not a directory |

## Step 30: Reading S-expressions

| Primitive | Status |
|---|---|
| `__Sexp_read__` | implemented |

* Goal: Start ConSh scripts fast. ConSh reads the syntax tree that `consh_parse --pos` writes, one S-expression per line; reading it in ConMa one character at a time (`Sexp_read` in `consh/include/consh_ast.se`) took about 3 seconds for a script of 850 characters.
* `__Sexp_read__ onError String ,(value)` reads one S-expression in the interpreter, with the same values and error messages as `Sexp_read`. ConSh now reads the tree in no measurable time; `Sexp_read` stays, and the ConSh test `step1` checks that both give the same results.
* Tests (`test/step30`):

| Case | What it checks |
|---|---|
| case1 | lists, the empty list, symbols that start with a digit (Numbers) and other symbols and strings (Strings) |
| case2 | the escapes in strings |
| case3 | `onError` with the message and column for an unterminated string and list, characters after the S-expression, an invalid number, and a missing value |

## Step 31: Trapping Ctrl-C

| Primitive | Status |
|---|---|
| `__OS_trap_interrupt__` | implemented |

* Goal: Let the interactive mode of ConSh stop only the running command on Ctrl-C. Without it, SIGINT raised Python's `KeyboardInterrupt` and ended the interpreter with a traceback.
* `__OS_trap_interrupt__ Ref ,()` installs a SIGINT handler that sets the value of `Ref` to a new Number. Signals already wake `select()` in the scheduler through the wakeup pipe of SIGCHLD (`signal.set_wakeup_fd`), so a VProc waiting for the Ref wakes up even when every other VProc waits for the OS.
* Tests (`test/step31`):

| Case | What it checks |
|---|---|
| case1 | an external command sends SIGINT to the interpreter twice; a VProc waiting for the Ref wakes up with 1, the value is 2 after the second, and the interpreter keeps running |

## Step 32: Starting processes outside the terminal's process group

| Primitive | Status |
|---|---|
| `__OS_spawn_detached__` | implemented |

* Goal: Let the interactive mode of ConSh keep the external commands of `&` jobs running when Ctrl-C stops the foreground command. All processes started with `__OS_spawn__` are in the process group of the interpreter, which is the foreground process group of the terminal, so a Ctrl-C at the terminal sends SIGINT to them all.
* `__OS_spawn_detached__` takes the same arguments as `__OS_spawn__` and starts the process in a new session (`start_new_session` of Python's `subprocess`). Moving the foreground process group of the terminal with `tcsetpgrp` is not needed: foreground commands stay in the group of the interpreter.
* Tests (`test/step32`):

| Case | What it checks |
|---|---|
| case1 | a child of `__OS_spawn__` is in the process group of the interpreter, and a child of `__OS_spawn_detached__` is in another one |

## Step 33: Fixes from the review of ConSh

| Primitive | Status |
|---|---|
| `__VProc_kill__` | changed: waits until the VProc has evaluated the LazyLists it claimed |
| `__OS_dup__` | implemented |
| `__OS_kill__` | fixed: an infinite number or NaN goes to `onError` |
| `__Sexp_read__` | fixed: no recursion limit; a string ending in `\` is reported at the end of the input |

* Goal: Fix what the review of ConSh (`consh/REVIEW-ja.md`, 2026-11-05) found in ConMa.
  * Stopping a VProc while it evaluated a LazyList (a channel element, a line of standard input) left the LazyList without a cell, so every later reader got an error; in the interactive mode of ConSh, Ctrl-C during `stdin` broke the shell's standard input. `__VProc_kill__` now marks such a VProc (`kill_pending`) and stops it when it has evaluated all the LazyLists it claimed with `__LazyList_claim__` (`evaluating_lazy`). LazyLists made by `__LazyList_hold__` do not delay it.
  * ConSh closes the file of a redirection at the end of the statement, but `&` jobs and `$( )` started in it may still write to it. `__OS_dup__ onError Stream ,(Stream)` duplicates the file descriptor, so that each child VProc gets its own Stream and closes it when it ends.
  * `__OS_kill__` with an infinite signal number and `__Sexp_read__` with about 1000 levels of nesting raised Python exceptions and ended the interpreter.
  * A VProc that waited only for the Ref of `__OS_trap_interrupt__` was reported as a deadlock; the scheduler now sleeps in `select` for the signal.
* Tests (`test/step33`):

| Case | What it checks |
|---|---|
| case1 | a VProc stopped while it evaluates a LazyList finishes the evaluation, and the value stays readable; the code after the evaluation does not run |
| case2 | a Stream from `__OS_dup__` can be written after the original is closed; `onError` for a closed Stream and a non-Stream |
| case3 | `__OS_kill__` with an infinite number and NaN, `__Sexp_read__` with 5000 levels and with a string ending in `\` |
| case4 | a VProc waiting only for the Ref of `__OS_trap_interrupt__` wakes up on SIGINT instead of a deadlock |

## Step 34: `__Cont_get__` as a primitive

| Primitive | Status |
|---|---|
| `__Cont_get__` | implemented (was a ConMa definition in `include/util.se`) |

* Goal: Make the early `return` of ConMa functions cheap. A `__Cont_get__` and its `return` took more than 30 steps; in ConSh, an `echo` in a `for` loop took about 4,360 steps, about 39% of them in `__Cont_get__` (about 51 calls per `echo`).
* `__Cont_get__ ,(return)` is a primitive: it takes the LCont (or, without one, pops the top CFrame), and passes a `return` that keeps the CChain. Applying `return` sets the CChain back, pops its top CFrame and applies the Closure to the operands, in one step. See PRIMITIVES.md.
* The ConMa definition moved from `include/util.se` to `include/control.se`. A program that includes `control.se` uses it instead of the primitive, because only global variables without a definition (`Undefined`) are looked up among the primitives. It is meant for following the steps as ConMa functions and for comparing the two versions.
* Every test gives the same output with either version, except the step numbers in the error messages of `test/step20` (case10, case15) and the largest number of live line cells in `test/step21` (case5, case6), which `memcheck.py` samples every 2000 steps. These logs were updated. With `(include "control.se")` added to `util.se`, all tests pass with their logs before this Step, `step1` traces included.
* ConSh: a `for` loop of 1000 `echo` commands takes 2,601,741 steps instead of 4,360,473 (about 2,600 steps per `echo` instead of 4,360), and runs in 4.6 seconds instead of 7.4 (without starting the interpreter).
* Tests (`test/step34`): each case runs both with the primitive (`test.se`) and with the ConMa version (`conma.se`, which includes `control.se` and `test.se`), and both must give `caseN.log`.

| Case | What it checks |
|---|---|
| case1 | `return` passes its values to the caller's LCont, and returns without values when it has none |
| case2 | a `return` applied in another function (the function given to `each`) returns from the function that called `__Cont_get__` |
| case3 | `__Cont_get__` without an LCont takes the top CFrame as its LCont |
| case4 | the LCont of an application of `return` is not used; `__type_name__` of `return` is `Closure` |
| case5 | `return` applied again after its function has returned goes back to the same LCont again |
| case6 | `return` in the first function of a VProc finishes the VProc, even with values |

## Step 35: `__if_then_else__`, `__if__` and `__unless__` as primitives

| Primitive | Status |
|---|---|
| `__if_then_else__` | implemented (was a ConMa definition in `include/pure.se`) |
| `__if__` | implemented (was a ConMa definition in `include/pure.se`) |
| `__unless__` | implemented (was a ConMa definition in `include/pure.se`) |

* Goal: Make conditionals cheap. Counting the steps of ConSh by function (`CONMA_PROFILE` of the Rust VM, `test/vm_profile.py`) showed that one iteration of `for i in 1 2 ...; do n=1; done` called `__if_then_else__` 88 times, and that the conditionals took about 40% of its steps.
* The three are primitives (see "Control Primitives" in PRIMITIVES.md). A Boolean (`,(t f) __NULL__ t` or `,(t f) __NULL__ f`, as the primitives return them and as `__TRUE__` and `__FALSE__` are) chooses its clause in one step, where the ConMa definitions took 7 (`__if_then_else__`) or 9 (`__if__`, `__unless__`). Any other value, such as a condition written as a function `(,() __is_null__ x)`, is applied to the two clauses as before, which saves only 1 or 3 steps.
* The ConMa definitions moved from `include/pure.se` to `include/control.se`, as `__Cont_get__` did in Step 34.
* In ConSh and in the ConMa front end, the 222 conditions of the form `(,() __NULL__ x)`, which only pass a Boolean on, are written `x`, so that they are chosen in one step.
* Every test gives the same output, except the step numbers in the error messages of `test/step15` (case3), `test/step17` (case3, case9) and `test/step20` (case10, case15), the largest number of live line cells in `test/step21` (case5, case6), which `memcheck.py` samples every 2000 steps, and the trace of `test/step1` (case2), where `__if_then_else__` is now a primitive. These logs were updated.
* ConSh (steps of one iteration, counted over 2000): `for i in 1 2 ...; do n=1; done` takes 681 steps instead of 759, and `for i in $(seq 1 2000); do n=$i; done` 1609 instead of 1824; the example `consh/examples/lazy.sh` takes 509,091 steps instead of 580,441.
* Tests (`test/step35`): each case runs both with the primitives (`test.se`) and with the ConMa versions (`conma.se`, which includes `control.se` and `test.se`), and both must give `caseN.log`.

| Case | What it checks |
|---|---|
| case1 | Booleans returned by a primitive choose the clause; `__if__` and `__unless__` do nothing when they do not choose it |
| case2 | `__TRUE__` and `__FALSE__` of `pure.se`, and the results of `__not__`, `__and__` and `__or__` |
| case3 | conditions written as functions that return a Boolean |
| case4 | a Boolean written in place, a partially applied Boolean, and a Closure that computes before it chooses (not of the form of a Boolean) |
| case5 | the chosen clause returns its values to the LCont; `__if__` that does not choose returns no values; `return` in a clause returns from the function that called `__Cont_get__` |
| case6 | a conditional in a condition, and recursion through `__unless__` |

## Step 36: Data primitives for ConSh

| Primitive | Status |
|---|---|
| `__Dict_copy__` | implemented |
| `__LList_append__` | implemented |
| `__LList_length__` | implemented |
| `__String_find_any__` | implemented |
| `__String_strip_suffix__` | implemented |

* Goal: Make the data operations that ConSh repeats for every statement cheap. Counting the steps of ConSh by function (`CONMA_PROFILE`, `test/vm_profile.py`) showed that copying its context Dict took 27% of the steps of a script that calls a function in a loop, and that checking words for glob characters, appending lists and removing newlines took 10 to 15% of a loop of `echo`.
* They work only on data and never apply a Closure, so each is one step (see the LList, Dict and String Primitives in PRIMITIVES.md). The ConMa versions are in `include/data.se`, like `include/control.se` for Step 35: a program that includes it uses them instead of the primitives. With a wrong type of argument, the ConMa versions stop with the error of the primitive they use, not with the same error.
* ConSh uses them in place of its own `Dict_copy`, `List_append` and `List_length`, and in `Glob_has_meta`, `Glob_has_any`, `Ast_chomp` and `Line_chomp`. With `CONSH_CONMA_DATA=1`, `consh/bin/consh` builds a module that includes `include/data.se` (`/tmp/consh.data.module.se`), and every ConSh test gives the same output with it.
* ConSh (Rust VM, steps of the benchmarks of `consh/PLAN-ja.md`, stage 18): `y=$(f $i)` 3000 times takes 11,988,797 steps instead of 17,550,750 (0.63 s instead of 0.96 s), `echo $i` 5000 times 4,933,016 instead of 5,565,174, and `for i in 1 2 ... 20000; do n=1; done` 4,898,524 instead of 5,798,940.
* Tests (`test/step36`): each case runs both with the primitives (`test.se`) and with the ConMa versions (`conma.se`, which includes `data.se` and `test.se`), and both must give `caseN.log`. The traces of the Python VM and the Rust VM are the same.

| Case | What it checks |
|---|---|
| case1 | `__Dict_copy__` keeps the order of the keys, and changing the copy or the original does not change the other; copying an empty Dict |
| case2 | `__LList_append__` with empty lists on either side, and the original list is unchanged; `__LList_length__` |
| case3 | `__String_find_any__` from several positions, with no match, no characters, at the end, in an empty String and after non-ASCII characters |
| case4 | `__String_strip_suffix__` with and without the suffix, `\r\n` in two steps, an empty suffix, a suffix longer than the String, and the whole String |

## Step 37: Opening a file and recording it in one step

| Primitive | Status |
|---|---|
| `__OS_open_tracked__` | implemented |

* Goal: A VProc can be killed between any two of its steps, so a program that opens a file and then records it (to close it when the VProc is killed) leaves the file open if the VProc is killed in between. ConSh records the files that redirections open in a Ref of its context (`"openRef"`), and closes them when a job is killed; in `consh/test/step13` case1 the file of a killed job sometimes stayed open (with some `CONMA_SEED` values, and often with transpiled code), because the job was killed after opening the file of `for ... done > file &` and before recording it.
* `__OS_open_tracked__ onError path mode Ref ,(Stream)` opens the file as `__OS_open__` does and, in the same step, puts the Stream in front of the LList in the Ref (see PRIMITIVES.md). ConSh uses it for the files of redirections: `Eval_redirs` takes the length of the table before it starts (`Open_mark`), and the statement closes the table back to it, instead of recording the opened Streams after `Eval_redirs` returns (`Open_track`).
* Tests (`test/step37`; the files are made in `$tmp`, which `test.sh` passes in `STEP37_DIR`):

| Case | What it checks |
|---|---|
| case1 | two files opened into the same Ref: the newer is first; writing through the recorded Streams and closing them |
| case2 | a file that cannot be opened, a value that is not a Ref, a Ref of a String and a wrong mode go to `onError`, and the Ref is unchanged |
| case3 | a VProc that opened a file into a Ref and then waits is killed; the file is in the Ref, and is closed from it |

## Step 38: Numbers without identity

* Goal: Numbers were objects with an identity, as in Python: every arithmetic result was another Number, and `__Ref_wait_change__`, which compares values by identity, woke a VProc when another VProc set an equal Number. Only ConSh used this: cancelling a channel (`Chan_cancel_chains` in `consh_chan.se`) set the count of consumed elements to an equal Number to wake the writers waiting for it to change; it now sets it to -1, which no count is. It also kept a Number from being only an `f64`, which the ConMa-to-Rust transpiler needs to keep Numbers in Rust variables.
* A Number is now only its value (`Value::Number(f64)` in the Rust VM). `__Ref_wait_change__` compares Numbers by value, with `+0` and `-0` the same and NaN the same as NaN (`same` in the Rust VM, `same_value` in the Python VM); every other value, including Strings, is still compared by identity. Dict and PDict keys keep their rule (Numbers by value, NaN equal to nothing).
* Tests (`test/step38`):

| Case | What it checks |
|---|---|
| case1 | setting an equal Number does not wake a VProc waiting with `__Ref_wait_change__`; setting another one does |
| case2 | a VProc waiting with another NaN on a Ref of NaN, and with `-0` on a Ref of `0`, waits until another value is set |
| case3 | a String with the same text is another String: setting it wakes the waiting VProc |

## Step 39: Square root

| Primitive | Status |
|---|---|
| `__Number_sqrt__` | implemented |

* Goal: The benchmarks that compare ConMa with CPython (`bench/`) need a square root (nbody), which Python has in `math.sqrt` (C in CPython); written in ConMa with Newton's method it would make ConMa do several times as much work.
* `__Number_sqrt__ onError Number ,(Number)` returns the square root; a negative Number (not `-0`) applies `onError`, as `math.sqrt` raises an error (see PRIMITIVES.md). The transpiled code calls it in place (`CHAIN` in `vm/conma2rust/conma2rust.py`).
* Tests (`test/step39`):

| Case | What it checks |
|---|---|
| case1 | the roots of 4, 2, 0, -0, 1e300, infinity and NaN |
| case2 | a negative Number goes to `onError` |

## Step 40: JavaScript in the VM for wasm32

| Primitive | Status |
|---|---|
| `__JS_call__` | implemented (JavaScript only in the VM for wasm32; the native VMs go to `onError`) |

* Goal: The builtin `js` of ConSh evaluates JavaScript in the browser (or Node.js) that runs the ConMa VM built for wasm32 (stages 4 to 6 of consh/js/PLAN-ja.md).
* `__JS_call__ onError code args ,(inputStream outputStream)` hands `code` and `args` to the host, which writes the result to a pipe as a line of an S-expression, after the lines of `console.log` and `console.error` and the requests for standard input, which the program answers through `outputStream` (see PRIMITIVES.md). In the VM for wasm32, `outputStream` is an fd whose writes and close go to the host (`host_js_input`, `host_js_input_end`). An iterator gives its elements one at a time, when the program asks for each with a line `n`, so that an endless generator gives only as many as are read. Waiting for the result is waiting for a Stream, so the scheduler is unchanged, and a Promise that takes time does not stop the other VProcs. In the VM for wasm32, `sys::js_call` (consh/js/wasm/src/sys.rs) calls the host's `host_js_call`, and the host gives the result with `conma_write` and `conma_close` after `conma_run` returns (consh/js/conma.js); in the native VMs, `sys::js_call` fails, and `__JS_call__` goes to `onError` after checking its arguments.
* Tests (`test/step40`; the logs are those of the native VMs: on the VM for wasm32, the calls of case1 get a Stream):

| Case | What it checks |
|---|---|
| case1 | correct arguments (Strings and nested LLists): `JavaScript is not available` |
| case2 | code that is not a String, args that are not an LList, and an element that is neither a String nor an LList, also inside a nested LList, go to `onError` |

## Step 41: Bitwise primitives of fast ConMa

| Primitive | Status |
|---|---|
| `__Number_and32__`, `__Number_or32__`, `__Number_xor32__`, `__Number_not32__`, `__Number_shl32__`, `__Number_shr32__`, `__Number_ushr32__` (feature `"bitwise32"` of fast ConMa) | implemented |

* Goal: ConMa had no bitwise operations, so richards (`bench/`) and the bitwise operators of SJS computed them with arithmetic, one bit at a time: about 30% of the steps of richards, and about 100 times the steps of an addition for an SJS `^`. Planned and built in docs/BITWISE-PLAN-ja.md (stages W1 to W7), the first primitives of fast ConMa (docs/FAST-PLAN-ja.md): concise ConMa is unchanged.
* The 32-bit meaning of JavaScript (PRIMITIVES.md, section 16). The fast VMs run them (the transpiled code computes them on `f64` in fused chains); the frontends (`bin/resolver`, `include/resolver.se`) mark a module that uses them `(Uses "bitwise32")`, and `bin/concise` puts in the replacement definitions of `include/ref/bitwise32.se` (`include/fast/bitwise32.se` until 2026-11-13) for the Python VM, which `test/vm.sh` does on its own.
* Tests: `test/step41` (the replacement definitions against the values of Node.js), `test/fast5` (the primitives in a module marked by the frontend, on every VM; the module translated by `bin/concise`; a module defining one of the names itself), `consh/sjs/test/step8` case9 (SJS).
* Results: richards 20 on the Rust VM 7.53 s to 5.24 s; one bitwise operation of SJS about 4,500 steps to about 65 (an addition takes 45).

## Planned: returning without making the escape continuation

Not built, and decided not to be built (2026-11-12, docs/FAST-PLAN-ja.md 4.4): on the Rust VM after F5, `__Cont_get__` takes 3.2% of the steps of the fast version of ConSh in consh/bench 02_for_seq_100000 (about 3% of its time at most), and only 48 of the 637 functions meet the conditions below. Moved here from consh/SAFEJS-PLAN-ja.md (stage B8), since it concerns ConMa and not only SJS, after the review in consh/SAFEJS-REVIEW-ja.md (3.1.4).

* Goal: when a function takes its escape continuation with `__Cont_get__ ,(return)` at its start and uses `return` only to return from itself, return through the CChain as a normal return does, without making the value of `return` (a Closure that keeps the CChain). Most functions of ConSh have this form, and so would the functions that SJS is translated into.
* Now (see `__Cont_get__` in PRIMITIVES.md): `__Cont_get__` makes `return`, which keeps the CChain at that time; applying it sets the CChain back to the kept one, then pops its top CFrame and applies its Closure. `return` is a value: it can be passed to other functions, put in a Ref, or called after the function has returned.
* Conditions, decided from the syntax of the function:
    1. `return` is bound by the `__Cont_get__ ,(return)` at the start of the body.
    2. `return` appears only as the operator of an application (`return x`), never as an operand (`f return`, or inside a Closure given as an operand such as `(,(msg) ... return)`), nor stored in a Ref or a Dict.
    3. Each application of `return` is in the body or in an LCont of the body (the `,(x) ...` at the end of an application), not in a Closure given as an operand, which another function might call when the CChain is deeper.
    4. Nothing takes the CChain out while the function runs: no `__CChain_get__`, `__CChain_pop_LCont__` or `__CChain_set__` in the function or in what it calls. This does not follow from 1 to 3: the `shift` of ConSh takes an LCont out of the CChain and applies it later on another CChain (`$k`), and then the CChain at the time of `return` is not the one at the time of `__Cont_get__`, so that the present `return` (back to the kept CChain) and the optimized one (to the top of the current CChain) differ. Since what a function calls cannot be told from its syntax alone, the optimization is limited to code known not to take continuations out, for example code made by a translator (SJS) whose runtime does not use the CChain primitives, or functions marked so.
* Under these conditions the CChain when `return` is applied is the one at `__Cont_get__` (after the callees have returned through their LConts), so `return x` can be "pop the top CFrame and apply it to x", as a function returns normally.
* Where: a pass of the front end (the resolver or the compiler in include/) that rewrites such applications into a new instruction or into existing CChain primitives (`__CChain_pop_LCont__` and an application); a new instruction would go into every VM (the Python VM, the Rust VM, the transpiled VM and the VM for wasm32). Which one is to be decided by measuring.
* Measured (2026-11-11, consh/sjs/README-ja.md): `__Cont_get__` takes 3.0% of the steps of the SJS loop benchmark and 2.3% of a ConSh script, so this would save about 5% at most; of the 637 functions of consh/include and include/ that take `__Cont_get__ ,(return)`, 48 meet conditions 2 and 3, the others applying `return` in a clause of `__if__` (which runs with the frame of its own LCont on the CChain, so the optimized return would have to drop it too). Left as planned for now.
* What can be seen: traces no longer show `(__Cont__ "n")` for such functions, and the number of steps changes; tests of test/ whose logs show steps would need a switch to turn it off, or new logs.
