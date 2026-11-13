# ConMa Built-in Primitives Reference

This document defines the standard built-in primitives available in the ConMa environment. All primitives follow the Continuation Passing Style (CPS) and are named with a `__` prefix.

## 1. OS Interaction Primitives

These primitives manage external processes and stream-based I/O.

### `__OS_get_envDict__ ,(envDict)`
* **Description**: Retrieves the environment variables of the current interpreter process.
* **Returns**: A new Dict (String → String) copied from the current environment variables. Changing the Dict does not change the interpreter's environment.
* **Usage**: Typically used to capture the parent environment before modifying it to pass into `__OS_spawn__`.

### `__OS_spawn__ onError commandMSeq inputStream outputStream errorStream envDict ,(OSProc)`
* **Description**: Launches an external operating system process with a specified environment.
* **Arguments**:
    * `commandMSeq`: An `MSeq` of Strings where the first element is the executable and subsequent elements are arguments. If the executable contains `/`, it is used as a path; otherwise it is searched for in the `PATH` of `envDict`, as `execvpe` does.
    * `inputStream`: An input Stream (the read end of a pipe) for the process's standard input, or null to inherit the interpreter's standard input.
    * `outputStream`: An output Stream (the write end of a pipe) for the process's standard output, or null to inherit the interpreter's standard output.
    * `errorStream`: An output Stream for the process's standard error, or null to inherit the interpreter's standard error.
    * `envDict`: A Dict (String → String) defining environment variables for the spawned process. The environment of the new process is exactly envDict.
* **Behavior**:
    * The process runs in the current directory of the current VProc (see `__OS_chdir__`).
    * Before the process starts, the interpreter writes out its own buffered standard output and standard error, so that they come before the process's output.
    * The Streams are not closed. The program closes the Streams it no longer needs with `__OS_close__`; for example, a reader of the process's output sees EOF only after every copy of the write end, including the program's own, has been closed.
    * When the interpreter exits, processes that are still running are left running; they are neither waited for nor killed.
* **Returns**: An `OSProc` object representing the spawned process.
* **Error**: It invokes the continuation specified by `onError` with an error message string if `commandMSeq` is not a non-empty MSeq of Strings, a Stream is of the wrong direction or closed, `envDict` is not a Dict of Strings, or the process cannot be started (for example, the executable is not found or not executable).


### `__OS_spawn_detached__ onError commandMSeq inputStream outputStream errorStream envDict ,(OSProc)`
* **Description**: Like `__OS_spawn__`, but the process starts in a new session (`setsid`).
* **Behavior**: The process is not in the process group of the terminal, so a Ctrl-C (SIGINT) or Ctrl-Z typed at the terminal does not reach it, and it has no controlling terminal: reading the terminal does not stop it with SIGTTIN, and closing the terminal does not send it SIGHUP. Waiting, polling and signals work as for `__OS_spawn__`.
* **Usage**: The interactive mode of ConSh starts the external commands of `&` jobs with it, so that Ctrl-C stops only the foreground command.
### **`__OS_poll__ onError OSProc ,(status)`**
* **Description**: Checks whether the process has terminated (non-blocking).
* **Returns**: If the process has terminated, returns its exit code Number; otherwise, returns `null`. If a signal terminated the process, the exit code is the negative signal number (for example, `-9`). Once the process has terminated, every later call returns the same exit code.
* **Usage**: To wait for a process without using the CPU, use `__OS_wait__`. Calling `__OS_poll__` repeatedly also works (other VProcs run between the calls), but it keeps the interpreter busy.
* **Error**: If an error occurs, it invokes the continuation specified by `onError` with an error message string.

### **`__OS_wait__ onError OSProc ,(status)`**
* **Description**: Waits until the process has terminated and returns its exit code Number, in the same form as `__OS_poll__`.
* **Behavior**: If the process has already terminated, it returns at once. Otherwise the current VProc waits, and other VProcs keep running. When no VProc is ready, the interpreter sleeps until a process exits, without using the CPU.
* **Error**: If `OSProc` is not an OSProc, invokes `onError` with an error message string.

### **`__OS_wait_readable__ onError inputStream ,()`**
* **Description**: Waits until `inputStream` has data or is at EOF, so that the next `__OS_read__` does not return `""`.
* **Behavior**: If the Stream is already readable, it returns at once. Otherwise the current VProc waits, as with `__OS_wait__`. It also wakes up if another VProc closes the Stream; the next `__OS_read__` then goes to `onError`.
* **Error**: If `inputStream` is not an input Stream or is closed, invokes `onError` with an error message string.


### **`__OS_wait_writable__ onError outputStream ,()`**
* **Description**: Waits until `outputStream` can take at least `PIPE_BUF` bytes (512 or more) without blocking, so that the next `__OS_write__` of that many bytes or fewer does not block the interpreter.
* **Behavior**: If the Stream is already writable, it returns at once. Otherwise the current VProc waits, as with `__OS_wait__`, and other VProcs keep running. It also returns when the reader of a pipe has closed it (the next `__OS_write__` then goes to `onError`), and when another VProc closes the Stream.
* **Usage**: `__OS_write__` blocks the whole interpreter while the OS cannot take the data. A VProc that writes to a pipe read by a process whose output another VProc reads should write in pieces of at most 512 bytes, each after `__OS_wait_writable__`.
* **Error**: If `outputStream` is not an output Stream or is closed, invokes `onError` with an error message string.
### `__OS_kill__ onError OSProc signal ,()`
* **Description**: Sends the signal `signal` (a Number from 1 to 64, such as 15 for SIGTERM) to the process.
* **Behavior**: Nothing happens if the process has already been waited for with `__OS_wait__` or `__OS_poll__`.
* **Error**: Invokes `onError` with an error message string if `OSProc` is not an OSProc, `signal` is not an integer from 1 to 64 (also for an infinite number or NaN), or the OS cannot send the signal.


### `__OS_dup__ onError Stream ,(Stream)`
* **Description**: Passes a new Stream on a duplicate of the file descriptor of `Stream` (`dup`), with the same mode, to the LCont.
* **Behavior**: Closing either Stream does not close the other; the open file stays open until both are closed. A program can give each user of an open file its own Stream to close, as the OS does for child processes.
* **Error**: Invokes `onError` with an error message string if `Stream` is not a Stream, is closed, or the OS cannot duplicate it.
### `__OS_getpid__ ,(Number)`
* **Description**: Passes the process ID of the interpreter to the LCont. Every VProc gets the same number.

### `__OS_listdir__ onError path ,(names)`
* **Description**: Passes the names in the directory `path` to the LCont, as an `LList` of Strings in code point order. `"."` and `".."` are not included; names that start with `.` are.
* **Behavior**: A relative `path` is relative to the current directory of the current VProc. The names have no directory part.
* **Error**: Invokes `onError` with an error message string if `path` is not a String, does not exist, or is not a directory.

### `__OS_trap_interrupt__ Ref ,()`
* **Description**: From now on, SIGINT (Ctrl-C) does not stop the interpreter. Instead, each SIGINT sets the value of `Ref` to a new Number, the number of SIGINTs received since this call.
* **Behavior**: The VProcs waiting for `Ref` with `__Ref_wait_change__` wake up, also while the interpreter sleeps in `select` waiting for the OS. Processes started with `__OS_spawn__` keep the default action of SIGINT, so a Ctrl-C at a terminal stops the external commands in its foreground process group as usual.
* **Usage**: The interactive mode of ConSh stops only the running command on Ctrl-C.

### `__OS_sysconf__ onError name ,(Number)`
* **Description**: Passes the value of the system configuration variable `name` to the LCont. `name` is a String with one of the names of `sysconf(3)`, such as `"SC_ARG_MAX"` (the maximum bytes of the arguments and environment of a new process) or `"SC_OPEN_MAX"`.
* **Error**: Invokes `onError` with an error message string if `name` is not a String, is not a known name, or the OS has no value for it.

### `__OS_exit_normal__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 0.

### `__OS_exit_error__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 1.

### **`__OS_exit__ onError Number ,()`**
* **Description**: Terminates the current OS Process (that is, the interpreter) with the exit code `Number`, even if other VProcs are still running.
* **Behavior**: As with `__OS_exit_normal__` and `__OS_exit_error__`, the output already written is flushed. A shell uses it to exit with the status of its last command.
* **Error**: If `Number` is not an integer from 0 to 255, invokes `onError` with an error message string. A non-Number argument stops the VProc with an error.

### `__OS_get_String__ ,(String)`
Behavior:
Gets a `String` from standard input and passes it to the LCont.
It reads through its own buffer, so a program should not mix it with reading the Stream from `__OS_get_stdin__`.

### `__OS_get_stdin__ ,(inputStream)` / `__OS_get_stdout__ ,(outputStream)` / `__OS_get_stderr__ ,(outputStream)`
* **Description**: Passes the interpreter's own standard input (an input Stream), standard output or standard error (output Streams) to the LCont.
* **Returns**: The same Stream each time it is called, so closing it is seen by every later call.
* **Behavior**: These Streams can be used with `__OS_read__`, `__OS_write__`, `__OS_close__` and `__OS_spawn__` like a pipe end. Writing to standard output or standard error keeps the order with `__OS_print__` and `__OS_print_error__`. Closing one marks the Stream as closed, but the interpreter keeps the OS file descriptor open for its own output.

### `__OS_print__ String ,()`
Behavior:
Writes the `String` to standard output.
No newline is appended.
After execution, it resumes the **LCont** with an empty OList — no values are passed to the continuation.

### `__OS_print_error__ String ,()`
Behavior:
Writes the `String` to standard error.
No newline is appended.
After execution, it resumes the **LCont** with an empty OList — no values are passed to the continuation.

### `__OS_pipe__ ,(inputStream outputStream)`
* **Description**: Creates a new unidirectional pipe.
* **Returns**: An OList containing two elements: the `inputStream` (read end) and the `outputStream` (write end).

### `__OS_read__ onError inputStream ,(data)`
* **Description**: Reads available data from the `inputStream`.
* **Returns**: The `data` read, which can be one of the following:
    * **`"..."` (Non-empty string)**: The actual data retrieved from the stream.
    * **`""` (Empty string)**: No data is currently available, but the stream remains open (the writer has not closed the pipe).
    * **`null`**: End of File (EOF). The stream has been closed by the writer.
* **Error**: If a system-level read error occurs, it invokes the continuation specified by `onError` with an error message string.

### `__OS_write__ onError outputStream data ,()`
* **Description**: Writes `data` to the `outputStream`.
* **Behavior**: Upon completion, resumes the continuation with an empty OList.
* **Error**: If the write fails, invokes `onError` with an error message string.

### **`__OS_close__ onError stream ,()`**
* **Description**: Closes the specified `stream` (input or output).
* **Behavior**: Upon completion, resumes the continuation with an empty OList.
* **Error**: If the stream cannot be closed (e.g., already closed or invalid handle), invokes `onError` with an error message string.

### **`__OS_open__ onError path mode ,(Stream)`**
* **Description**: Opens the file `path`, relative to the current directory of the current VProc.
* **Behavior**: `mode` is one of the following Strings:
    * **`"r"`**: Opens the file for reading and returns an input Stream.
    * **`"w"`**: Creates the file, or empties it if it exists, and returns an output Stream.
    * **`"a"`**: Creates the file if it does not exist and returns an output Stream that writes at the end of the file.
* **Returns**: The Stream. It is used with `__OS_read__`, `__OS_write__`, `__OS_close__` and `__OS_spawn__` like a pipe end.
* **Error**: If `path` or `mode` is not a String, `mode` is not one of the above, `path` is a directory, or the OS cannot open the file (for example, it does not exist or permission is denied), invokes `onError` with an error message string.

### **`__OS_open_tracked__ onError path mode Ref ,(Stream)`**
* **Description**: Opens the file `path` as `__OS_open__` does and, in the same step, records the Stream in `Ref`.
* **Behavior**: On success, the value of `Ref` (an LList or null) becomes the LList with the Stream in front of it, as `__LList_cons__` would make it, before the Stream is returned. A VProc is killed only between two steps, so a program that records the files it opens (to close them if the VProc is killed) never has a file open but not recorded. VProcs waiting for the Ref to change (`__Ref_wait_change__`) are woken as by `__Ref_set__`.
* **Returns**: The Stream.
* **Error**: As `__OS_open__`, and if `Ref` is not a Ref whose value is an LList or null, invokes `onError` with an error message string; then no file is opened and the Ref is unchanged.

### **`__OS_chdir__ onError path ,()`**
* **Description**: Changes the current directory of the current VProc to `path`.
* **Behavior**: Each VProc has its own current directory, an absolute path. The first VProc starts in the directory where the interpreter was started, and a VProc made by `__VProc_spawn__` starts in the current directory of the VProc that made it; after that, `__OS_chdir__` changes only the current VProc's. A relative `path` is relative to the current directory of the current VProc, and the new current directory is its real path (symbolic links resolved), as `__OS_getcwd__` of the OS gives. The current directory is used for relative paths in `__OS_open__`, `__OS_chdir__`, `__OS_realpath__` and the `sourceFileName` of `__VProc_spawn__`, and as the working directory of processes started by `__OS_spawn__`. The interpreter's own current directory does not change.
* **Error**: If `path` is not a String, or the OS cannot change to it (for example, it does not exist or is not a directory), invokes `onError` with an error message string.

### **`__OS_getcwd__ ,(String)`**
* **Description**: Passes the absolute path of the current directory of the current VProc to the LCont.

### **`__OS_realpath__ onError path ,(String)`**
* **Description**: Returns the absolute path of `path`, with `.`, `..` and symbolic links resolved.
* **Behavior**: A relative `path` is relative to the current directory of the current VProc (see `__OS_chdir__`). Two paths name the same file if their real paths are equal, so this can be used to find a file that is given twice under different names.
* **Error**: If `path` is not a String, or does not exist (including the empty String and a path that goes through a missing directory, such as `none/..`), invokes `onError` with an error message string.

### **`__JS_call__ onError code args ,(inputStream outputStream)`**
* **Description**: Has the host of the VM evaluate the JavaScript `code` (a String) with `args` (an LList whose elements are Strings and LLists of the same kind). Returns an input Stream from which the result is read, and an output Stream whose writes go to the code as its standard input.
* **Behavior**: Only the VM for wasm32 (`consh/js`) has JavaScript. It makes a pipe, gives its write end to the host and returns its read end; what is written to `outputStream` goes to the host, and closing it ends the standard input of the code. The host evaluates `code` as an expression in its global scope, where `stdin` names the standard input; if the value is a function, it calls it with `args` (Strings, and arrays for LLists); it waits for a Promise. It writes lines of S-expressions that `__Sexp_read__` reads, and then closes the pipe. The last line is the result: `(r s "text")` for a string (a number, a boolean, a BigInt or another object is made into one with `String()` or `JSON.stringify`), `(r l (...))` for an array (an element that is an array is a list; `null` and `undefined` are `""`), `(r n)` for `undefined` and `null`, or `(e "message")` for an exception (`"Name: message"` for an Error). Lines before it are `(o "text")` and `(x "text")`, what the code wrote with `console.log` and `console.error`, and `(i)`, a request for the next element of standard input, which the program answers by writing that element to `outputStream` as a line, a String in double quotes, or the line `end` at its end. A result `(r t)` is an iterator (any iterable object but an array or a string): for each line `n` that the program writes to `outputStream`, the host writes the next element as `(t "text")` or `(t (...))`, or `(d)` at the end, or `(e "message")`. Closing `outputStream` tells the host that the program no longer reads: the host stops the call (an iterator with its `return()`), and forgets the JavaScript values that the call gave (`__JS_value__`). The VM closes `outputStream` also when the last reference to it is dropped, so that a program can keep it with the values it holds. The VProc reads the Stream as any other, so it waits for the result while the other VProcs run (see 2.1 and 2.2 of consh/js/PLAN-ja.md).
* **Usage**: The builtin `js` of ConSh.
* **Error**: If `code` is not a String, or `args` is not an LList of Strings and LLists, invokes `onError` with an error message string. The native VMs (`test/vproc_step.py` and `vm/`) have no JavaScript: with correct arguments, they invoke `onError` with `"__JS_call__: JavaScript is not available"`.

### **`__JS_value__ onError id ,(String)`**
* **Description**: Returns the JavaScript value numbered `id` (a String) in the host of the VM, as the S-expression of a value of ConSh that `__Sexp_read__` reads: `(s "text")` for a string (a number, a boolean, a BigInt or another object is made into one with `String()` or `JSON.stringify`, or `String()` when that fails), `(l (...))` for an array (as in a result of `__JS_call__`), `(n)` for `undefined` and `null`, and `(f)` for a function.
* **Behavior**: Only the VM for wasm32 (`consh/js`) has JavaScript. It asks the host at once and does not wait (wasm imports are synchronous), so the value is made from the object as it is now. The numbers are those that the host gives in the results of `js name= code` and of SJS, `(( ))` and `$(( ))` (see 2.5 and 3.23 of consh/js/PLAN-ja.md, and consh/SAFEJS-PLAN-ja.md).
* **Usage**: A shell variable of ConSh that holds a JavaScript value, when it is read with `$`.
* **Error**: If `id` is not a String, or the host has no value of that number, invokes `onError` with an error message string. The native VMs (`test/vproc_step.py` and `vm/`) have no JavaScript: with a String, they invoke `onError` with `"__JS_value__: JavaScript is not available"`.

---

## 2. Number Primitives

Primitives for floating-point arithmetic and numeric utilities.

### **`__Number_fromString__ onError String ,(Number)`**
* **Description**: Converts a `String` to a `Number`.
* **Returns**: The resulting `Number` object.
* **Error**: If the `String` is not a valid numeric representation, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

### **`__Number_toString__ Number ,(String)`**
* **Description**: Converts a `Number` to its string representation.

### **`__Number_is_zero__ Number ,(Boolean)`** / **`__Number_is_positive__`** / **`__Number_is_negative__`**
* **Description**: Evaluates the sign property of the given `Number` and returns a boolean value. `-0` is zero and neither positive nor negative.

### **`__Number_is_negative_zero__ Number ,(Boolean)`**
* **Description**: Returns true if `Number` is `-0`, and false for any other `Number` (`+0`, NaN, whatever its sign bit, and every nonzero Number). A `Number` is an IEEE 754 double, which can be `-0` (`__Number_fromString__` of `"-0"`, `-1` times `0`, `__Number_sqrt__` of `-0`), but the other primitives treat `-0` as `0` (`__Number_toString__` writes it as `"0"`, `__Number_is_negative__` is false for it, and dividing by it is an error like dividing by `0`); this is the only primitive that tells them apart. It is for languages translated to ConMa that need the sign of zero, such as JavaScript (`1 / -0` is `-Infinity`).
* **Error**: If the operand is not a `Number`, stops the VProc with an error, as `__Number_is_zero__` does.

### **`__Number_add__ aNumber bNumber ,(sumNumber)`** / **`__Number_subtract__ aNumber bNumber ,(differenceNumber)`** / **`__Number_multiply__ aNumber bNumber ,(productNumber)`**
* **Description**: Performs the corresponding arithmetic operation on two `Number` values and returns the resulting `Number`.

### **`__Number_divide__ onError aNumber bNumber ,(quotientNumber)`**
* **Description**: Divides `aNumber` by `bNumber`.
* **Returns**: The resulting `Number`.
* **Error**: If `bNumber` is zero, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

### **`__Number_sqrt__ onError Number ,(rootNumber)`**
* **Description**: Returns the square root of `Number` (the root of `-0` is `-0`, of infinity infinity, of NaN NaN).
* **Error**: If `Number` is negative, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

### **`__Number_floor__ Number ,(intNumber)`**
* **Description**: Returns the greatest integer less than or equal to the given `Number` as an integer-valued `Number`.

---

## 3. LList (Linked List) Primitives

Low-level operations for the fundamental linked list structure.

### **`__LList_cons__ head tail ,(newList)`**
* **Description**: Creates a new list node whose head is `head` and whose tail is `tail`, and returns it as `newList`.
* **Note**: This is the fundamental `cons` operation for constructing a linked list.

### **`__LList_uncons__ onError list ,(head tail)`**
* **Description**: Decomposes `list` into its head element and its tail (the remainder of the list).
* **Returns**: If `list` is not null and is a LList, returns two values: the `head` element and the `tail` (the remaining LList).
* **Error**: If `list` is null or not a `LList`, the interpreter sets the Operator to `onError` with an error message String.

### **`__LList_append__ a b ,(list)`**
* **Description**: Returns a new list of the elements of `a` followed by `b`. The cells of `a` are copied; `b` is not (the result ends with `b` itself). If `a` is null, returns `b`.
* **Usage**: One step instead of one function call for each element of `a` (Step 36). The ConMa version is in `include/data.se`.
* **Error**: If `a` is not a list that ends with null, the VProc stops with an error. `b` may be any value.

### **`__LList_length__ list ,(Number)`**
* **Description**: Returns the number of elements of `list` (0 for null).
* **Error**: If `list` is not a list that ends with null, the VProc stops with an error.

---

## 4. Continuation Chain (CChain) Primitives

Advanced primitives for manipulating the virtual process's control stack.

### `__CChain_get__ ,(CChain)`
Behavior:
Passes the current CChain to the LCont.

### `__CChain_set__ CChain ,()`
Behavior:
Sets the CChain as the current CChain.
It replaces the Virtual Process's existing `CChain` with the `CChain` passed as an argument.

**Operational Note:**
Unlike standard function applications, `__CChain_set__` does not push a new `CFrame` onto the chain.
The provided `LCont` is executed as the immediate successor to the primitive call, effectively resuming execution within the context of the newly set `CChain`.

**Virtual Process State and `__CChain_set__` Behavior**

A **Virtual Process (VProc)** independently maintains two primary control-flow structures: a **Continuation Chain (CChain)**, which represents the persistent sequence of pending computation frames, and a **Local Continuation (LCont)**, which represents the immediate next operation to be executed within the current context.

When the primitive `__CChain_set__ cc ,() ...` is executed, the interpreter performs the following atomic operations:

1. **CChain Replacement**: The current `CChain` of the VProc is entirely replaced by the provided `cc`.
2. **LCont Invocation**: The interpreter then immediately invokes the `LCont` defined at the call site (the closure `,() ...`).

**Operational Distinction:**
Unlike standard function applications, `__CChain_set__` does **not** push the current `LCont` onto the newly set `CChain`. The `LCont` is executed as the direct successor to the primitive call, effectively resuming execution from the "front" of the new execution context defined by `cc`.

### `__CChain_push_CFrame__ CFrame ,()`
Behavior:
Sets the `next CFrame` field of the given `CFrame` to the current top of the CChain, then sets the given `CFrame` as the new top of the CChain.
Invokes the current LCont with no argument

### `__CChain_pop_CFrame__ ,(CFrame)`
Behavior:
Pops a CFrame from the CChain and passes it to the LCont.
If the CChain is empty, passes null to the LCont instead.

### `__CChain_pop_LCont__ ,(LCont)`
Behavior:
pops a parent LCont and passes it to the current LCont.
It is defined as follow:
```
(define __CChain_pop_LCont__ ,()
  __CChain_pop_CFrame__ ,(cf)
  __CChain_pop_CFrame__ ,(cf2)
  __CFrame_get_Closure__ cf ,(lc)
  __CFrame_get_Closure__ cf2 ,(lc2)
  lc lc2)
```

When the expression
```
__CChain_pop_LCont__ ,(lc) ...
```
is executed, the VProc's state is:

- **Operator**: the closure of the function defined as `__CChain_pop_LCont__`
- **OList**: empty
- **LCont**: the closure of `,(lc) ...`

Because `__CChain_pop_LCont__` takes no parameters, the runtime pushes the `LCont` closure `,(lc) ...` onto the CChain as a new CFrame, then begins executing the body of `__CChain_pop_LCont__`. At this point the CChain contains, from top to bottom:

1. the CFrame for `,(lc) ...` — just pushed by this call
2. the CFrame of the caller of `__CChain_pop_LCont__` — already present before the call

The first `__CChain_pop_CFrame__` therefore pops the CFrame for `,(lc) ...`, binding it to `cf`.
The second `__CChain_pop_CFrame__` pops the caller's CFrame, binding it to `cf2`.

`__CFrame_get_Closure__` then extracts the closure from each:

- `lc` receives the closure of `,(lc) ...` — the LCont syntactically written at the call site.
- `lc2` receives the closure of the caller's CFrame — the continuation that was pending before `__CChain_pop_LCont__` was invoked.

Finally, `lc lc2` invokes `lc` with `lc2` as its argument, binding the caller's continuation to the variable `lc` at the call site and proceeding with `...`.

In short, two pops are required because the call to `__CChain_pop_LCont__` itself introduces one CFrame onto the chain, so the LCont written at the call site occupies the top of the chain, and the caller's original continuation sits one level beneath it.

### `__Cont_get__ ,(return)`
Behavior:
Passes `return`, the escape continuation of the function that calls `__Cont_get__`, to the LCont (see "Continuation" in [SPEC.md](SPEC.html)).
Without an LCont, the top CFrame of the CChain is popped and its Closure is the LCont; if the CChain is empty too, the VProc stops with an error.
`return` keeps the CChain as it is after the LCont is taken, whose top CFrame is the LCont of the caller of the function.
Applying `return` to any number of operands, in one step:
1. sets the CChain to the kept one,
2. pops its top CFrame and applies the Closure of the CFrame to all the operands (to none without operands).
If the kept CChain is empty, `return` empties the CChain instead, so the VProc finishes, and the operands are dropped.
The LCont of the application of `return` is never used.
`return` can be applied any number of times, also after the function has returned.

`__type_name__` gives `Closure` for `return`; in traces it is written `(__Cont__ "n")`, where `n` numbers the continuations made so far, as a Ref is written (the kept CChain may contain `return` itself).

**Fast ConMa only (since 2026-11-13):** `__Cont_get__` is a primitive of fast ConMa (the feature `"control"`, section 16). In concise ConMa it is the ConMa definition of `include/ref/control.se`, with the same behavior, on top of `__CChain_pop_LCont__`, `__CChain_get__` and `__CChain_set__`; a program of fast ConMa that includes `ref/control.se` uses that definition instead of the primitive.
A program that includes `control.se` uses that definition instead of the built-in one, because only global variables without a definition are looked up among the primitives (`Undefined` in [RESOLVER.md](RESOLVER.html)).
The two versions differ only in what can be seen of the steps: the ConMa version takes more than 30 steps for `__Cont_get__` and its `return` together, where the primitive takes 2 (one each), and its `return` is a variadic Closure with the CFrames of `control.se` in traces and in the CChain while it runs.

---

## 5. CFrame Primitives

### `__CFrame_new__ FuncExp SInfo ,(CFrame)`
Behavior:
Creates a CFrame and passes it to the LCont.
The CFrame consists of a Closure of `FuncExp`, the SInfo, and a null as the next CFrame.

### `__CFrame_get_Closure__ CFrame ,(Closure)`
Behavior:
Passes the Closure stored in `CFrame` to the LCont.

### `__CFrame_get_SInfo__ CFrame ,(String)`
Behavior:
Passes a string formed by concatenating the strings contained in the SInfo stored in `CFrame`, separated by colons to the LCont.

---

## 6. VProc Primitives

### `__VProc_spawn__ onError entryPoint sourceFileName errorSink argumentSeq inputSeq outputSink ,(VProc)`
Behavior:
Initiates the creation and execution of a new Virtual Process (VProc) through the following sequential phases:

1. **Process Initialization**: A new Virtual Process is allocated with its own CChain and Process Dictionary.
2. **Static Analysis and Loading**: The interpreter loads the program from the file specified by `sourceFileName`, relative to the current directory of the current VProc (see `__OS_chdir__`). The new VProc starts in the same current directory. During this loading phase, the interpreter performs a **global scan** of all `define` statements within the file and any included files.
3. **Environment Binding**: All identifiers identified during the scan are registered in the new program's GVEnv. This ensures that named functions can refer to each other recursively or out of order from the moment execution begins.
4. **Entry Point Execution**: The new VProc applies the global function named by the String `entryPoint` to the following arguments. A program normally uses `"__main__"`, which calls the program's `main` (see "Main" in SPEC.md).
* `errorSink`: For handling standard error output.
* `argumentSeq`: For program startup arguments.
* `inputSeq`: Representing the standard input stream. `__main__` takes a `LazyList` of lines (see "Main" in SPEC.md), but the primitive passes any value.
* `outputSink`: For standard output.

The new VProc is placed at the end of the ready queue, and the `VProc` is passed to the LCont.
The arguments are passed as they are: a closure passed from the parent keeps referring to the parent's program (its GVEnv), and the Refs and other values it refers to are shared.

* **Error**: If `sourceFileName` does not exist, loading it fails (for example, a syntax error), or `entryPoint` is not defined in the program, the interpreter invokes `onError` with an error message String, and no VProc is created. An error that occurs while the new VProc runs does not invoke `onError`.

### `__VProc_start__ Closure ,(VProc)`
Behavior:
Starts a new Virtual Process (VProc) that applies `Closure` to no arguments, and passes the `VProc` to the LCont.
Unlike `__VProc_spawn__`, it loads no file: the closure keeps referring to the program it was made in (its GVEnv), and the local variables, Refs and other values it refers to are shared with the VProc that made it.
The new VProc has its own CChain and Process Dictionary, starts in the current directory of the current VProc (see `__OS_chdir__`), and is placed at the end of the ready queue. It finishes when the closure returns to its empty CChain.
To get a result, the closure puts it into a Ref that the caller reads after `__VProc_wait__`.

* **Usage**: To run part of a program concurrently, for example each element of a shell pipeline, without the cost of loading a program file for each VProc.
* **Error**: Stops the current VProc with an error if `Closure` is not a Closure. An error that occurs while the new VProc runs finishes only the new VProc.

### `__VProc_id__ VProc ,(Number)`
Behavior:
Passes the number of `VProc` to the LCont: 1 for the first VProc, then 2, 3, ... in the order VProcs are made. It is the number that traces and error messages show.

* **Error**: Stops the current VProc with an error if `VProc` is not a VProc.

### `__VProc_kill__ VProc ,()`
Behavior:
Finishes `VProc`, wherever it is: it is removed from the ready queue and stops waiting (for the OS, another VProc, a LazyList or a Ref). If it is the current VProc, it stops after this primitive. A VProc that has already finished is left as it is.
If `VProc` is evaluating a `LazyList` that it claimed with `__LazyList_claim__`, it is not stopped at once: it keeps running until it has evaluated every such LazyList (published its cell, or left the evaluation with a continuation), and stops then (Step 33). So the LazyList stays readable by other VProcs. A VProc waiting inside such an evaluation (for input, for another VProc) therefore stops only after that wait ends.
The VProcs that wait for `VProc` with `__VProc_wait__` wake up when it stops. A `LazyList` made by `__LazyList_hold__` that `VProc` holds is left without a cell, so a VProc that claims it gets `onError` (see `__LazyList_claim__`).
OS processes that `VProc` started keep running; send them a signal with `__OS_kill__`.

* **Error**: Stops the current VProc with an error if `VProc` is not a VProc.

### `__VProc_current__ ,(VProc)`
Behavior:
Passes the current VProc to the LCont.

### `__VProc_suspend__ VProc ,()`
Behavior:
Suspends the `VProc`.
Moves the `VProc` from the ready queue to the blocked queue.
If the `VProc` is the current VProc, it stops after this primitive and, when it is resumed, continues with the LCont.
If the `VProc` is already blocked or has finished, nothing happens.

### `__VProc_resume__ VProc ,()`
Behavior:
Resumes the `VProc`.
Moves the `VProc` from the blocked queue to the end of the ready queue.
If the `VProc` is not blocked (it is ready, running, or has finished), nothing happens.

### `__VProc_wait__ onError VProc ,()`
Behavior:
The current VProc waits until `VProc` has finished, normally or with a runtime error, and then continues with its LCont. While it waits, it is not in the ready queue, and other VProcs keep running.
If `VProc` has already finished, it continues at once. Several VProcs may wait for the same `VProc`.
`__VProc_suspend__` and `__VProc_resume__` do not change a VProc that waits.
A VProc that waits for another VProc can only wake up when that one finishes: if no VProc is ready and none waits for the OS, the VProcs that wait for a VProc count as blocked for the deadlock check (see "Runtime Model" in SPEC.md).
The result of `VProc` is not returned. To get it, pass the child a Sink (for example, as `outputSink` of `__VProc_spawn__`) that puts its values into a Ref, and read the Ref after waiting; an empty Ref then means that the child finished without a result, for example with a runtime error.

* **Error**: If `VProc` is not a VProc, or is the current VProc, the interpreter invokes `onError` with an error message String.

---

## 7. PDict Primitives

### `__PDict_put__ key value ,()`
Behavior:
Associates the `value` with the `key` in the Process Dictionary.
Invokes the current LCont with no argument

### `__PDict_lookup_or__ key onMissing ,(value)`
Behavior:
Retrieves the value associated with `key` in the Process Dictionary.

* If the key exists, the associated value is passed to the current **LCont**.
* If the key does not exist, `onMissing` is invoked with `key` as its argument.

`onMissing` is invoked with the **same continuation** that `__PDict_lookup_or__` would have used for the successful case.
Therefore, if `onMissing` eventually calls that continuation, its return value becomes the result of the lookup.
However, if `onMissing` is itself a continuation (or otherwise does not invoke that continuation), control does not return to the call site.

Example:
```
__PDict_lookup_or__ key (,(key) __CChain_pop_LCont__ __NULL__) ,(value) ...
```
If no value is associated with `key`, the `onMissing` function invokes the same continuation with `__NULL__`, so `value` receives `__NULL__`.

Supplementary explanation:
When `onMissing` — the closure `(,(key) __CChain_pop_LCont__ __NULL__)` — is invoked, its body is:

```
__CChain_pop_LCont__ __NULL__
```

Here `__CChain_pop_LCont__` takes no parameters, so `__NULL__` is a surplus argument. Per the excess-argument rule, the runtime pushes a local continuation `(,(Sink) Sink __NULL__)` onto the CChain before entering the body of `__CChain_pop_LCont__`. At this point the CChain contains, from top to bottom:

1. the CFrame for `(,(Sink) Sink __NULL__)` — pushed due to the surplus argument
2. the CFrame for `,(value) ...` — the LCont of the `__PDict_lookup_or__` call site

`__CChain_pop_LCont__` then executes:

- The first `__CChain_pop_CFrame__` pops the CFrame for `(,(Sink) Sink __NULL__)`, binding its closure to `lc`.
- The second `__CChain_pop_CFrame__` pops the CFrame for `,(value) ...`, binding its closure to `lc2`.
- `lc lc2` invokes `(,(Sink) Sink __NULL__)` with `lc2` as its argument, binding `lc2` — the closure `,(value) ...` — to `Sink`.
- The body then evaluates `Sink __NULL__`, which applies `,(value) ...` to `__NULL__`, binding `__NULL__` to `value`.

The net result is that `value` receives `__NULL__`, exactly as if the lookup had succeeded and returned `__NULL__`.

---

## 8. Ref Primitives

### `__Ref_new__ value ,(Ref)`
Behavior:
Creates a Ref, sets the `value`, and passes it to the LCont.

### `__Ref_get__ Ref ,(value)`
Behavior:
Gets the `value` for the `Ref` and passes it to the LCont.

### `__Ref_set__ Ref value ,()`
Behavior:
Sets the `value` for the `Ref`.
Invokes the current LCont with no argument

### `__Ref_wait_change__ Ref value ,()`
* **Description**: Waits until the value of a `Ref` is another value.
* **Behavior**: If the value of `Ref` is `value` (the same value: for Numbers, an equal Number, with `+0` and `-0` the same and NaN the same as NaN, since Numbers have no identity; for any other value, including Strings, the same object), the current VProc waits (it is not in the ready queue) until another VProc sets another value, and then returns. Otherwise it returns at once. A VProc that reads the value, tests it, and then waits for it to change does not miss a change made in between, because it passes the value it read. A waiting VProc is counted as blocked for deadlock detection.
* **Error**: Stops the VProc with an error if `Ref` is not a `Ref`.

---

## 9. Null Primitives

### `__is_null__ Value ,(Boolean)`
Behavior:
Passes `__TRUE__` to the LCont if the `Value` is null, otherwise passes `__FALSE__`.

### `__NULL__`
A variable defined as null.

---

## 10. MSeq Primitives

### **`__MSeq_is_empty__ onError mseq ,(Boolean)`**

* **Description**: Takes an **mseq** and returns whether it is empty.
* **Returns**: `__TRUE__` if the mseq is an empty MSeq; `__FALSE__` if the mseq is a MSeq and it contains one or more elements.
* **Error**: `onError` is called with an error message String if the mseq is not a MSeq.

### **`__MSeq_uncons__ onError mseq ,(head tail)`**

* **Description**: Decomposes the **mseq** into its first element and the remaining sequence.
* **Returns**: If the mseq is a MSeq and not empty, returns two values: the **head** element and the **tail** (the remaining MSeq).
* **Error**: If the mseq is not a MSeq or is an empty MSeq, the Closure `onError` is invoked as an Operator Application with an error message String.

### **`__MSeq_set_head__ onError mseq value ,()`**

* **Description**: Performs a destructive update on the first position (index 0) of the **mseq**. An empty MSeq has no head, so nothing is created; see Error.
* **Behavior**: If the mseq is a MSeq and not empty, replaces the current **head** element with the provided `value` via an in-place update.
* **Error**: If the mseq is not a MSeq or is an empty MSeq, the Closure `onError` is invoked as an Operator Application with an error message String.
* **Returns**: Nothing (performs a side effect on the MSeq).

### **`__MSeq_append__ onError mseq value ,()`**

* **Description**: Appends a `value` to the end of the **mseq**, increasing the total element count.
* **Behavior**: If the mseq is a MSeq, appends `value` to the end of `mseq`.
* **Error**: If the mseq is not a MSeq, the Closure `onError` is invoked as an Operator Application with an error message String.
* **Returns**: Nothing (performs a side effect on the MSeq).

### **`__MSeq_new__ ,(MSeq)`**

* **Description**: Allocates and initializes a new, **empty MSeq** object.
* **Returns**: The newly created MSeq instance.

---

## 11. Dict Primitives

### **`__Dict_new__ ,(Dict)`**
* **Description**:
  Creates a new, empty Dict.
* **Returns**:
  A Dict containing no entries.

### **`__Dict_put__ Dict key value ,()`**
* **Description**:
  Associates `value` with `key` in the Dict.
* **Behavior**:
  * If `key` is not present, a new entry is created.
  * If `key` is already present, its associated value is **replaced** with `value`.
* **Returns**:
  Nothing (invokes the current LCont with an empty OList).

### **`__Dict_lookup_or__ Dict key onMissing ,(value)`**
* **Description**:
  Retrieves the value associated with `key`.
* **Behavior**:
  * If `key` exists, its associated value is passed to the current LCont.
  * If `key` does not exist, `onMissing` is invoked with `key` as its argument.
* **Continuation Rule**:
  * `onMissing` is invoked with the **same continuation** that would have been used if the lookup had succeeded.
  * If `onMissing` eventually invokes that continuation, its result becomes the result of the lookup.

### **`__Dict_remove__ Dict key ,()`**
* **Description**:
  Removes the entry associated with `key`.
* **Behavior**:
  * If `key` exists, the corresponding entry is removed.
  * If `key` does not exist, the operation has **no effect**.
* **Returns**:
  Nothing (invokes the current LCont with an empty OList).

### **`__Dict_keys__ Dict ,(Seq)`**
* **Description**:
  Produces a Sequence containing all keys in the Dict.
* **Behavior**:
  * The resulting Seq enumerates each key exactly once.
  * The order of keys is **implementation-defined**; a program should not rely on it. (`test/vproc_step.py` uses insertion order; see Step 8 in ROADMAP.md.)
* **Returns**:
  A Seq of keys.

### **`__Dict_copy__ Dict ,(Dict)`**
* **Description**: Returns a new Dict with the same keys and values as `Dict`, in the same order (the keys come from `__Dict_keys__` in that order). The values are not copied. Changing one Dict afterwards does not change the other.
* **Usage**: ConSh copies its context, a Dict of about 35 keys, when it calls a function or enters a loop; with `__Dict_keys__` and a Sink this took about 10 steps for each key (Step 36). The ConMa version is in `include/data.se`.
* **Error**: A non-Dict argument stops the VProc with an error.

---

## 12. String Primitives

Positions and lengths count Unicode code points. An escape sequence such as `\n` is one character. Positions start at 0.
A non-String argument (or, for `__String_substring__`, a non-Number position, and for `__String_fromCode__`, a non-Number) stops the VProc with an error.

### **`__String_length__ String ,(Number)`**
* **Description**: Returns the number of characters in the `String`.

### **`__String_substring__ onError String start end ,(String)`**
* **Description**: Returns the characters of the `String` from position `start` up to, but not including, position `end`.
* **Returns**: A new `String`. If `start` equals `end`, the empty String.
* **Error**: If `start` or `end` is not an integer, or `0 <= start <= end <= length` does not hold, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

### **`__String_find__ onError String pattern start ,(index)`**
* **Description**: Finds the first occurrence of the String `pattern` in `String` at or after position `start`.
* **Returns**: The position (a Number, counted in characters like `__String_substring__`) where `pattern` starts, or null if it does not occur. An empty `pattern` is found at `start`.
* **Usage**: Searching with one primitive instead of comparing one character at a time is much faster on long Strings; the line readers of `include/util.se` use it to find newlines.
* **Error**: If `start` is not an integer, or `0 <= start <= length` does not hold, the interpreter invokes `onError` with an error message string. A non-String `String` or `pattern` stops the VProc with an error.

### **`__String_find_any__ onError String chars start ,(index)`**
* **Description**: Finds the first character of `String` at or after position `start` that is one of the characters of `chars`.
* **Returns**: The position of that character (a Number), or null if there is none (also when `chars` is empty).
* **Usage**: ConSh checks every word for the glob characters `*`, `?` and `[` with one call (Step 36). The ConMa version (`include/data.se`) calls `__String_find__` for each character of `chars`.
* **Error**: As for `__String_find__`: if `start` is not an integer, or `0 <= start <= length` does not hold, invokes `onError` with an error message string; a non-String `String` or `chars` stops the VProc with an error.

### **`__String_strip_suffix__ String suffix ,(String)`**
* **Description**: If `String` ends with `suffix`, returns `String` without it; otherwise (and when `suffix` is empty) returns `String` itself.
* **Usage**: ConSh removes the newline (and a carriage return before it) from each line it reads (Step 36). The ConMa version is in `include/data.se`.
* **Error**: A non-String argument stops the VProc with an error.

### **`__Sexp_read__ onError String ,(value)`**
* **Description**: Reads the `String` as one S-expression and passes the value to the LCont.
* **Returns**: A list becomes an `LList` (the empty list `()` becomes null). A string in double quotes becomes a String; in it, `\n`, `\t` and `\r` stand for a newline, a tab and a carriage return, and `\` before any other character stands for that character (`\"`, `\\`). A symbol that starts with a digit becomes a Number; any other symbol becomes a String. Spaces, tabs and newlines may surround the S-expression and separate its items.
* **Usage**: ConSh reads the syntax tree that `consh_parse --pos` writes, one S-expression per line, with it. Reading the same text in ConMa one character at a time took seconds for a script of a few hundred characters.
* **Behavior**: Lists are read with an explicit stack, so any depth of nesting can be read.
* **Error**: If the String is not one well-formed S-expression, invokes `onError` with `"<message> at column <n>"`, where the message is `unterminated list`, `unterminated string`, `expected a value`, `invalid number` (a symbol that starts with a digit but is not a number) or `unexpected characters after an S-expression`, and `<n>` counts from 1. A non-String argument stops the VProc with an error.

### **`__String_concat__ aString bString ,(String)`**
* **Description**: Returns a new `String` made of `aString` followed by `bString`.

### **`__String_toCode__ onError String ,(Number)`**
* **Description**: Returns the Unicode code point of the one character of `String`.
* **Error**: If `String` does not have exactly one character, the interpreter invokes `onError` with an error message string.

### **`__String_fromCode__ onError Number ,(String)`**
* **Description**: Returns a `String` of one character, whose code point is `Number`. It is the inverse of `__String_toCode__`, and can make characters that a String literal cannot write, such as control characters.
* **Error**: If `Number` is not an integer from 0 to 0x10FFFF (1114111), or is a surrogate code point (0xD800 to 0xDFFF), the interpreter invokes `onError` with an error message string. A non-Number argument stops the VProc with an error.

### **`__String_equal__ aString bString ,(Boolean)`**
* **Description**: Returns `__TRUE__` if the two Strings have the same content, `__FALSE__` otherwise. A String made at run time equals a String literal with the same content.

---

## 13. Type Primitives

### **`__type_name__ Value ,(String)`**
* **Description**: Returns the name of the type of `Value`.
* **Returns**: One of the following Strings:

| Value | Result |
|---|---|
| null (including an empty LList and an empty CChain) | `"null"` |
| a String, a literal or one made at run time | `"String"` |
| a Number | `"Number"` |
| a Closure or a primitive | `"Closure"` |
| a Ref, CFrame, LList, MSeq, Dict, VProc or OSProc | `"Ref"`, `"CFrame"`, `"LList"`, `"MSeq"`, `"Dict"`, `"VProc"`, `"OSProc"` |
| a Stream, for input or output | `"Stream"` |

* **Behavior**: A type is tested by comparing the result with `__String_equal__`:
```
__type_name__ value ,(t)
__String_equal__ t "String" ,(isString)
__if_then_else__ isString ...
```
* **Limitations**:
    * A Boolean, a Seq and a Sink are Closures, so they give `"Closure"`.
    * A non-empty CChain is an LList of CFrames, so it gives `"LList"`.
* **Error**: None. Every value has a type, so there is no `onError`.

## 14. LazyList Primitives

A `LazyList` (see "LazyList" in SPEC.md) is a `Ref` whose value is a thunk (a Closure), null (an evaluated empty list), an `LList` cell (head, and a `LazyList` as tail), or the `VProc` that is evaluating the thunk or holds the `LazyList`. These primitives read and set that value, each in one Operator application, so that several VProcs may evaluate the same `LazyList`. The other `LazyList` functions are defined in `include/util.se`.

With the mark, the interpreter keeps the CChain of the VProc at the claim (not a ConMa value). The VProc has **left the evaluation** when that CChain is no longer the bottom of its current CChain: a continuation has taken it out of the thunk. A `LazyList` made by `__LazyList_hold__` is never left.

### **`__LazyList_claim__ onError LazyList ,(thunk cell)`**
* **Description**: Starts the evaluation of a `LazyList`, or gets its cell.
* **Behavior**:
    * If the value is a thunk, sets the current VProc as the value and returns the thunk and null. The caller applies the thunk and sets the cell with `__LazyList_publish__`.
    * If the value is null or an `LList`, returns null and the value.
    * If the value is a VProc that has not finished but has left the evaluation, claims the `LazyList` again as if its value were the thunk: sets the current VProc as the value and returns the thunk and null.
    * If the value is another VProc that has not finished and has not left the evaluation, the current VProc waits (it is not in the ready queue) until the value is no longer such a VProc, and then runs `__LazyList_claim__` again.
* **Error**: The interpreter invokes `onError` with an error message String if `LazyList` is not a `Ref`, or is a `Ref` of another kind of value, if the value is the current VProc and it has not left the evaluation (the `LazyList` depends on itself, or this VProc holds it and has not published it yet), or if the value is a VProc that has finished without setting the cell.

### **`__LazyList_hold__ ,(LazyList)`**
* **Description**: Makes a `LazyList` that has no value yet and is held by the current VProc.
* **Behavior**: Returns a new `LazyList` whose value is the current VProc. Unlike a claimed `LazyList`, it is held wherever the VProc goes, until the VProc sets its cell with `__LazyList_publish__`. Another VProc that claims it waits; if the holding VProc finishes first, it gets `onError`. `__LazyList_pipe__` uses it for the tail that has no value yet.
* **Error**: None.

### **`__LazyList_publish__ LazyList cell ,()`**
* **Description**: Sets the cell of a `LazyList` that the current VProc has claimed or holds.
* **Behavior**: Sets `cell` as the value. The VProcs waiting for the `LazyList` in `__LazyList_claim__` wake up at the next scheduling step.
* **Error**: Stops the VProc with an error if `LazyList` is not a `Ref`, if its value is not the current VProc, or if `cell` is neither null nor an `LList`.

## 15. Control Primitives

`__if_then_else__`, `__if__` and `__unless__` are primitives of fast ConMa (the feature `"control"`, section 16, since 2026-11-13) that do what their ConMa definitions in `include/ref/control.se` do (see "Control flow" in SPEC.md), in fewer steps. In concise ConMa they are those definitions: `include/ref/pure.se` includes `control.se`, and the Python VM has no such primitives.
A program of fast ConMa that includes `ref/control.se` uses those definitions instead, because only global variables without a definition are looked up among the primitives (`Undefined` in [RESOLVER.md](RESOLVER.html)).

A **Boolean** here is a Closure that is one of the two Closures `,(t f) __NULL__ t` and `,(t f) __NULL__ f`, whatever its parameters are named: two parameters (left after a partial application), a Body whose Operator is null, whose OList is the first or the second parameter, and without an LCont.
The Booleans returned by the primitives, and `__TRUE__` and `__FALSE__` of `include/ref/pure.se`, are of this form. In fast ConMa, the primitives `__TRUE__` and `__FALSE__` of the feature `"boolean"` (section 16) are Booleans too, and a module with that feature gets them from the primitives instead.

### **`__if_then_else__ Boolean then else`**
* **Description**: Applies `then` or `else`, as `Boolean` chooses, with the LCont.
* **Behavior**:
    * If `Boolean` is a Boolean, the chosen clause becomes the Operator, without operands and with the LCont of the application, in one step.
    * Otherwise, as the ConMa definition, `Boolean` is applied to `then` and `else`, and the Closure it passes back is applied: the LCont goes to the CChain (as when a Closure with an LCont is applied), `Boolean` becomes the Operator with the operands `then` and `else`, and the LCont is `,(clause) clause`. This is how a condition written as a function, such as `(,() __is_null__ x)`, is evaluated.
    * Operands after the third are dropped.
* **Error**: Stops the VProc with an error if there are fewer than three operands.

### **`__if__ Boolean then`** / **`__unless__ Boolean else`**
* **Description**: `__if_then_else__ Boolean then __nop__` and `__if_then_else__ Boolean __nop__ else`.
* **Behavior**: As `__if_then_else__`, with the Closure `,()` as the other clause. When that clause is chosen for a Boolean, the primitive returns to the LCont without values in the same step, as `__nop__` does.
* **Error**: Stops the VProc with an error if there are fewer than two operands.

**Steps:** with a Boolean, choosing takes one step besides the clause, where the ConMa definitions take 7 (`__if_then_else__`) or 9 (`__if__`, `__unless__`); when `__if__` or `__unless__` chooses the clause that does nothing, that one step is all. A condition written as a function still has to be applied, so it saves only 1 step (`__if_then_else__`) or 3 (`__if__`, `__unless__`): to choose in one step, compute the Boolean first (`__is_null__ x ,(b) __if__ b ...`). The two versions differ only in the steps and in what traces show; with too few operands, the ConMa definitions are partially applied instead of stopping with an error, and surplus operands, which the primitives drop, are applied to what the definitions return (an `__if__` written with two clauses, which the primitive ran as if the second were not there, fails with the definition: `consh/include/sjs_gen.se` had one, found when concise ConMa got the definitions).

---

## 16. Primitives of Fast ConMa

These primitives are not part of concise ConMa, the language defined above and in docs/SPEC.md. They belong to **fast ConMa** (docs/FAST-PLAN-ja.md): a module that uses one of them as a global without a definition is marked by the frontend with `(Uses "feature")` after its `(F ...)`. The fast VMs (Rust, wasm, transpiled) run the primitives; the concise VM (Python) refuses such a module, and `bin/concise` translates it into concise ConMa by putting in the replacement definitions of the feature, functions of concise ConMa with the same names and meaning (`test/vm.sh` does this for the Python VM). A global that a module defines is used before a primitive of the same name on every VM.

### Feature `"bitwise32"`: bitwise operations (docs/BITWISE-PLAN-ja.md)

The 32-bit bitwise operations of JavaScript. Each operand is made a 32-bit integer with ToInt32: NaN and infinities become 0, other Numbers are truncated toward 0 and taken modulo 2^32, and the result is read as a signed integer. The results are signed 32-bit integers (`__Number_ushr32__` unsigned). A shift count is the low 5 bits of the ToUint32 of the right operand. There are no errors besides operands that are not Numbers, which stop the VProc. Replacement definitions: `include/ref/bitwise32.se`.

### **`__Number_and32__ a b ,(Number)`** / **`__Number_or32__`** / **`__Number_xor32__`**
* **Description**: `a & b`, `a | b`, `a ^ b` of JavaScript.

### **`__Number_not32__ a ,(Number)`**
* **Description**: `~a` of JavaScript.

### **`__Number_shl32__ a n ,(Number)`** / **`__Number_shr32__`** / **`__Number_ushr32__`**
* **Description**: `a << n`, `a >> n` (sign-propagating) and `a >>> n` (zero-fill, an unsigned result) of JavaScript.

### Feature `"boolean"`: the Booleans as primitives (docs/FAST-PLAN-ja.md 4.6)

In concise ConMa, `__TRUE__` and `__FALSE__` are the Closures `,(then else) __NULL__ then` and `,(then else) __NULL__ else`, defined in `include/ref/pure.se`. A module of fast ConMa is made with `include/fast` before `include/` (`conma build`, `bin/update-frontend`, `bench/module.sh`, ConSh), where `include/fast/pure.se` does not define them, so they are globals without a definition, and the frontend writes `(Uses "boolean")`. Modules of concise ConMa are made with `include/ref` before `include/` (test/resolver.sh, the Python VM). Replacement definitions: `include/ref/pure.se`.

In a module with this feature, the Booleans the primitives return (`__is_null__`, the Number predicates, `__String_equal__` and the others) are these primitives too, not the Closures. `__type_name__` gives `"Closure"` for both, and `__if_then_else__`, `__if__` and `__unless__` take both.

### Feature `"control"`: the control functions as primitives (docs/FAST-PLAN-ja.md 4.6)

`__if_then_else__`, `__if__`, `__unless__` (section 15) and `__Cont_get__` (section 4). In concise ConMa they are the ConMa definitions of `include/ref/control.se`, which `include/ref/pure.se` includes; `include/fast/pure.se` does not, so a module of fast ConMa uses them without a definition, and the frontend writes `(Uses "control")`. Replacement definitions: `include/ref/control.se`.

### **`__TRUE__ then else ,(then)`** / **`__FALSE__ then else ,(else)`**
* **Description**: Passes `then` (`__TRUE__`) or `else` (`__FALSE__`) to the LCont, without applying it, in one step (the Closure takes two).
* **Error**: Stops the VProc with `__TRUE__: missing argument(s)` with fewer than two operands, where the Closure would be applied partially. Operands after the second are dropped, as by other primitives.

