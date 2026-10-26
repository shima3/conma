# Primitive Implementation Roadmap

This document gives an order for implementing (or re-implementing) the built-in primitives defined in [PRIMITIVES.md](PRIMITIVES.html).
Each step depends only on the steps before it, so the primitives in a step can be tested with what is already available.
The test directory for a step is `test/stepN`. Existing step directories are `step1` to `step19`.

Status shows whether `test/vproc_step.py` currently has the primitive (as of 2026-10-26).

## Tests

Each `test/stepN/test.sh` builds `test.se` with `test/resolver.sh`, runs each case function with `test/vproc_run.py` (`test/step13` also starts programs with `--main`), and compares the output with `caseN.log`. From `step2` on, `exit: <code>` is appended to the output; `step1` compares the full `--trace` output instead. No output means every case passed.

```
for t in module1 module2 step1 step2 step3 step4 step5 step6 step7 step8 step9 step10 step11 step12 step13 step14 step15 step16 step17 step18 step19; do test/$t/test.sh; done
```

* Standard output and standard error go to the same log, so `PYTHONUNBUFFERED=1` keeps their order.
* Standard input is empty (`/dev/null`) unless a case needs it (`test/step9` case5 reads `input.txt`; `test/step13` gives several cases an input file).
* Cases that write files (`test/step14`) write only in a `work/` directory that `test.sh` empties before each case and removes at the end. Absolute paths of the test directory are printed as `<dir>`, so that the logs do not depend on where the repository is.
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
* The current directory belongs to the interpreter, so `__OS_chdir__` changes it for every VProc. It is used for relative paths in `__OS_open__`, `__OS_chdir__` and `__VProc_spawn__`, and as the working directory of `__OS_spawn__`.
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

No new primitive; the language and `__Cont_get__` (in `include/util.se`) change.

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
* `__String_repr__ String ,(String)` quotes the String as Python's `repr` does: in single quotes, or in double quotes if it has `'` and no `"`; `\`, the quote, newline, tab and carriage return are escaped, and characters that Python does not print are written as `\xNN`, `\uNNNN` or `\UNNNNNNNN`. A character is not printed if it is in the categories Cc, Cf, Zs, Zl, Zp or Co (except the space), as `str.isprintable` of Python decides; the ranges of these characters are written in `util.se` for Unicode 15.1 (Python 3.13). Unassigned characters (category Cn, such as U+FFFF) are written as they are, whereas Python escapes them.
* Tests (`test/step19`):

| Case | What it checks |
|---|---|
| case1 | `__String_toCode__`: ASCII, escape sequences, Japanese text, a character outside the BMP |
| case2 | `__String_fromCode__`: the inverse of `__String_toCode__`; the result equals a String literal; U+0000 is a String of length 1 |
| case3 | `onError` for a String of length 0 and 2, a non-integer, a negative Number, 0x110000 and the two ends of the surrogates; 0x10FFFF is accepted |
| case4, case5 | a non-String and a non-Number argument stop the VProc with an error |
| case6 | `__String_repr__`: the choice of quotes; escapes of `\`, the quote, newline, tab and carriage return; Japanese text is not escaped |
| case7 | `__String_repr__` of the characters at the ends of the ranges that are not printed, and next to them; an unassigned character is not escaped |
