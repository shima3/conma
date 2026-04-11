# ConMa Built-in Primitives Reference

This document defines the standard built-in primitives available in the ConMa environment. All primitives follow the Continuation Passing Style (CPS) and are named with a `__` prefix.

## 1. OS Interaction Primitives

These primitives manage external processes and stream-based I/O.

### `__OS_spawn__ commandMSeq inputStream outputStream errorStream ,(OSProc)`
* **Description**: Launches an external process. The `commandMSeq` is an MSeq where the first element is the executable path and subsequent elements are its command-line arguments. `inputStream`, `outputStream`, and `errorStream` are connected to the process's standard I/O handles.
* **Returns**: An `OSProc` object representing the spawned process.

### `__OS_exit_normal__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 0.

### `__OS_exit_error__ ,()`
Behavior:
Terminates the current OS Process (that is, the interpreter) with an exit code of 1.

### `__OS_get_String__ ,(String)`
Behavior:
Gets a `String` from standard input and passes it to the LCont.

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

---

## 2. Number Primitives

Primitives for floating-point arithmetic and numeric utilities.

### **`__Number_fromString__ onError String ,(Number)`**
* **Description**: Converts a `String` to a `Number`.
* **Returns**: The resulting `Number` object.
* **Error**: If the `String` is not a valid numeric representation, the interpreter invokes `onError` as the next operator with a single error message string as its argument.

### **`__Number_toString__ Number ,(String)`**
* **Description**: Converts a `Number` to its string representation.

### **`__Number_is_zero__ Number ,(Bool)`** / **`__Number_is_positive__`** / **`__Number_is_negative__`**
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
* **Error**: If `list` is null or not a `LList`, the interpreter sets the Operator to `onError` and the OList to a single error message string instead of performing the decomposition.

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

### `__VProc_spawn__ errorSink sourceFileName argumentSeq inputSeq outputSink`
Behavior:
Initiates the creation and execution of a new Virtual Process (VProc) through the following sequential phases:

1. **Process Initialization**: A new, isolated Virtual Process is allocated with its own independent memory, CChain, and Process Dictionary.
2. **Static Analysis and Loading**: The interpreter loads the program from the file specified by `sourceFileName`. During this loading phase, the interpreter performs a **global scan** of all `define` statements within the file and any included files.
3. **Environment Binding**: All identifiers identified during the scan are registered in the new VProc’s `VEnv`. This ensures that named functions can refer to each other recursively or out of order from the moment execution begins.
4. **Entry Point Execution**: The interpreter invokes the `main` function of the loaded program by applying the `__main__` wrapper.
5. **Context Injection**: The `__main__` function is applied to the following arguments:
* `errorSink`: For handling standard error output.
* `argumentSeq`: For program startup arguments.
* `inputSeq`: Representing the standard input stream.
* `outputSink`: For standard output.

### `__VProc_current__`
Behavior:
Passes the current VProc to the LCont.

### `__VProc_suspend__ VProc`
Behavior:
Suspends the `VProc`.
Moves the `VProc` from the ready queue to the blocked queue.

### `__VProc_resume__ VProc`
Behavior:
Resumes the `VProc`.
Moves the `VProc` from the blocked queue to the ready queue.

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
Passes `(,(t f) __NULL__ t)` to the LCont if the `Value` is null, otherwise passes `(,(t f) __NULL__ f)`.

### `__NULL__`
A variable defined as null.

---

## 10. MSeq Primitives

### **`__MSeq_is_empty__ onError mseq ,(Boolean)`**

* **Description**: Takes an **mseq** and returns whether it is empty.
* **Returns**: `__TRUE__` if the mseq is an empty MSeq; `__FALSE__` if the mseq is a MSeq and it contains one or more elements.
* **Error**: `onError` is called if the mseq is not a MSeq.

### **`__MSeq_uncons__ onError mseq ,(head tail)`**

* **Description**: Decomposes the **mseq** into its first element and the remaining sequence.
* **Returns**: If the mseq is a MSeq and not empty, returns two values: the **head** element and the **tail** (the remaining MSeq).
* **Error**: If the mseq is not a MSeq or is an empty MSeq, the Closure `onError` is invoked as an Operator Application with no operands.

### **`__MSeq_set_head__ onError mseq value ,()`**

* **Description**: Performs a conditional destructive update on the first position of the **mseq**. The head position is defined as index 0; if it does not exist, it is created.
* **Behavior**: If the mseq is a MSeq and not empty, replaces the current **head** element with the provided `value` via an in-place update.
* **Error**: If the mseq is not a MSeq or is an empty MSeq, the Closure `onError` is invoked as an Operator Application with no operands.
* **Returns**: Nothing (performs a side effect on the MSeq).

### **`__MSeq_append__ onError mseq value ,()`**

* **Description**: Appends a `value` to the end of the **mseq**, increasing the total element count.
* **Behavior**: If the mseq is a MSeq, appends `value` to the end of `mseq`.
* **Error**: If the mseq is not a MSeq, the Closure `onError` is invoked as an Operator Application with no operands.
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
  * The order of keys is **implementation-defined**.
* **Returns**:
  A Seq of keys.
