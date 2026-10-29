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
    * The process runs in the interpreter's current directory (see `__OS_chdir__`).
    * Before the process starts, the interpreter writes out its own buffered standard output and standard error, so that they come before the process's output.
    * The Streams are not closed. The program closes the Streams it no longer needs with `__OS_close__`; for example, a reader of the process's output sees EOF only after every copy of the write end, including the program's own, has been closed.
    * When the interpreter exits, processes that are still running are left running; they are neither waited for nor killed.
* **Returns**: An `OSProc` object representing the spawned process.
* **Error**: It invokes the continuation specified by `onError` with an error message string if `commandMSeq` is not a non-empty MSeq of Strings, a Stream is of the wrong direction or closed, `envDict` is not a Dict of Strings, or the process cannot be started (for example, the executable is not found or not executable).

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

### `__OS_exit_normal__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 0.

### `__OS_exit_error__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 1.

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
* **Description**: Opens the file `path`, relative to the current directory.
* **Behavior**: `mode` is one of the following Strings:
    * **`"r"`**: Opens the file for reading and returns an input Stream.
    * **`"w"`**: Creates the file, or empties it if it exists, and returns an output Stream.
    * **`"a"`**: Creates the file if it does not exist and returns an output Stream that writes at the end of the file.
* **Returns**: The Stream. It is used with `__OS_read__`, `__OS_write__`, `__OS_close__` and `__OS_spawn__` like a pipe end.
* **Error**: If `path` or `mode` is not a String, `mode` is not one of the above, `path` is a directory, or the OS cannot open the file (for example, it does not exist or permission is denied), invokes `onError` with an error message string.

### **`__OS_chdir__ onError path ,()`**
* **Description**: Changes the interpreter's current directory to `path`.
* **Behavior**: Every VProc shares the current directory. It is used for relative paths in `__OS_open__`, `__OS_chdir__` and the `sourceFileName` of `__VProc_spawn__`, and as the working directory of processes started by `__OS_spawn__`.
* **Error**: If `path` is not a String, or the OS cannot change to it (for example, it does not exist or is not a directory), invokes `onError` with an error message string.

### **`__OS_getcwd__ ,(String)`**
* **Description**: Passes the absolute path of the current directory to the LCont.

### **`__OS_realpath__ onError path ,(String)`**
* **Description**: Returns the absolute path of `path`, with `.`, `..` and symbolic links resolved.
* **Behavior**: A relative `path` is relative to the current directory (see `__OS_chdir__`). Two paths name the same file if their real paths are equal, so this can be used to find a file that is given twice under different names.
* **Error**: If `path` is not a String, or does not exist (including the empty String and a path that goes through a missing directory, such as `none/..`), invokes `onError` with an error message string.

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
* **Description**: Evaluates the sign property of the given `Number` and returns a boolean value.

### **`__Number_add__ aNumber bNumber ,(sumNumber)`** / **`__Number_subtract__ aNumber bNumber ,(differenceNumber)`** / **`__Number_multiply__ aNumber bNumber ,(productNumber)`**
* **Description**: Performs the corresponding arithmetic operation on two `Number` values and returns the resulting `Number`.

### **`__Number_divide__ onError aNumber bNumber ,(quotientNumber)`**
* **Description**: Divides `aNumber` by `bNumber`.
* **Returns**: The resulting `Number`.
* **Error**: If `bNumber` is zero, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

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
2. **Static Analysis and Loading**: The interpreter loads the program from the file specified by `sourceFileName`, relative to the current directory (see `__OS_chdir__`). During this loading phase, the interpreter performs a **global scan** of all `define` statements within the file and any included files.
3. **Environment Binding**: All identifiers identified during the scan are registered in the new program's GVEnv. This ensures that named functions can refer to each other recursively or out of order from the moment execution begins.
4. **Entry Point Execution**: The new VProc applies the global function named by the String `entryPoint` to the following arguments. A program normally uses `"__main__"`, which calls the program's `main` (see "Main" in SPEC.md).
* `errorSink`: For handling standard error output.
* `argumentSeq`: For program startup arguments.
* `inputSeq`: Representing the standard input stream. `__main__` takes a `LazyList` of lines (see "Main" in SPEC.md), but the primitive passes any value.
* `outputSink`: For standard output.

The new VProc is placed at the end of the ready queue, and the `VProc` is passed to the LCont.
The arguments are passed as they are: a closure passed from the parent keeps referring to the parent's program (its GVEnv), and the Refs and other values it refers to are shared.

* **Error**: If `sourceFileName` does not exist, loading it fails (for example, a syntax error), or `entryPoint` is not defined in the program, the interpreter invokes `onError` with an error message String, and no VProc is created. An error that occurs while the new VProc runs does not invoke `onError`.

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

A `LazyList` (see "LazyList" in SPEC.md) is a `Ref` whose value is a thunk (a Closure), null (an evaluated empty list), an `LList` cell (head, and a `LazyList` as tail), or the `VProc` that is evaluating the thunk. These two primitives read and set that value, each in one Operator application, so that several VProcs may evaluate the same `LazyList`. The other `LazyList` functions are defined in `include/util.se`.

### **`__LazyList_claim__ onError LazyList ,(thunk cell)`**
* **Description**: Starts the evaluation of a `LazyList`, or gets its cell.
* **Behavior**:
    * If the value is a thunk, sets the current VProc as the value and returns the thunk and null. The caller applies the thunk and sets the cell with `__LazyList_publish__`.
    * If the value is null or an `LList`, returns null and the value.
    * If the value is another VProc that has not finished, the current VProc waits (it is not in the ready queue) until the value is no longer such a VProc, and then runs `__LazyList_claim__` again.
* **Error**: The interpreter invokes `onError` with an error message String if `LazyList` is not a `Ref`, or is a `Ref` of another kind of value, if the value is the current VProc (the `LazyList` depends on itself), or if the value is a VProc that has finished without setting the cell.

### **`__LazyList_publish__ LazyList cell ,()`**
* **Description**: Sets the cell of a `LazyList` that the current VProc has claimed.
* **Behavior**: Sets `cell` as the value. The VProcs waiting for the `LazyList` in `__LazyList_claim__` wake up at the next scheduling step.
* **Error**: Stops the VProc with an error if `LazyList` is not a `Ref`, if its value is not the current VProc, or if `cell` is neither null nor an `LList`.
