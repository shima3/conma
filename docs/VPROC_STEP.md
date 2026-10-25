### **Specification of the ConMa VProc Interpreter (`vproc_step.py`)**

#### **1. Purpose and Operational Context**
The `vproc_step.py` engine implements the **Single-Step Application** of a VProc. Unlike a traditional recursive evaluator, `step()` assumes that the **Operator** has already been resolved into a terminal value. Its primary role is to execute the transition between these values and manage the continuation structures.

#### **2. Execution Model: The `step` Function**
The transition logic in `step(vp, gvenv)` is driven by the type of the **Operator** register. The resolution of Variables and FuncExps into Closures is performed during the *transition to the next state* (via `eval_expr`), not within the `step` dispatch itself.

* **Null Handling & Continuation Resumption**:
    * If the Operator is **NULL**, the VProc attempts to resume a pending computation (checking `LCont`, then `CChain`).

* **SeqClosure Application (Partial Application & Context Restoration)**:
    When the Operator is a **SeqClosure**, it acts as a mechanism to preserve both the lexical environment and any buffered arguments from a previous over-application.
    * **State Restoration**: In both execution branches below, the VProc’s **VEnv** is restored to the environment captured by the SeqClosure (`operator.lvenv`), and the VProc’s **OList** is replaced by the SeqClosure's internal argument buffer (`operator.olist`).
    * **Branch 1: Argument Consumption (If the current OList is NOT empty)**:
        * The first element of the current **OList** is promoted to the new **Operator**.
        * This effectively "feeds" a new argument into the buffered context restored from the SeqClosure.
    * **Branch 2: Continuation Resumption (If the current OList IS empty)**:
        * The interpreter pops a **CFrame** from the **CChain**.
        * The closure saved within the CFrame becomes the new **Operator**.
        * The source information (`SInfo`) is also restored from the CFrame.

* **Primitive Dispatch**:
    If the Operator is not a Closure or NULL, the interpreter handles it through two distinct paths:
    * **Direct Callables**: If the Operator is a native Python `callable` (typically injected into `gvenv` during the loading phase via the `PRIMITIVES` registry), it is executed directly.
    * **Named Primitives (PrimFunc)**: If the Operator is a structure tagged as `["PrimFunc", name]`, the interpreter performs a **runtime lookup** using `globals().get(name)`. If a corresponding Python function exists in the script's global scope, it is executed. 
    > *Note: Runtime `PrimFunc` dispatch relies entirely on the Python symbol table (`globals`), not the `PRIMITIVES` dictionary.*

* **Closure Application**:
    * When the **Operator** is a **Closure**, the interpreter applies it to the current **OList** based on the following three sub-cases, determined by the presence of parameters in the closure's `Head` and values in the `OList`.

* **Case 0: Application of a Variadic Closure**
    If the closure is variadic (`Closure.variadic`, made from a Head `(Head pos Rest (Variable ...))`):
    1.  **LCont Preservation**: If the current **LCont** is present, it is pushed onto the **CChain** as a new **CFrame**.
    2.  **Binding**: The whole current **OList** (possibly empty) is wrapped into a **SeqClosure** (capturing the current `VEnv`), which is bound to the closure's one parameter.
    3.  **Body Execution**: The closure's `Body` is extracted in the extended `VEnv` and executed as in Case 1. The closure is executed even when the OList is empty, and there are no surplus operands.
    In traces, the Head of a variadic closure is written `(Head (0 0) Rest (Variable (0 0) "args"))`, and it is read back as variadic.

* **Case 1: Application of a Parameterless Closure (Zero-arity)**
    If the closure's `Head` is empty (no parameters):
    1.  **LCont Preservation**: If the current **LCont** is present, it is pushed onto the **CChain** as a new **CFrame**.
    2.  **Over-argument Handling**: If the current **OList** is not empty, its contents are wrapped into a **SeqClosure** (capturing the current `VEnv`) and pushed onto the **CChain**.
    3.  **Body Execution**: The closure's `Body` is extracted and its elements are dispatched to the VProc registers (`Operator`, `OList`, `LCont`, `SInfo`). Global variables in the Body are resolved in the closure's own `gvenv` (see section 5), and the `gvenv` argument of `step` is used only for closures that have none.

    * **Body Extraction**: The interpreter processes the closure's `Body` node. Rather than relying on a fixed element order, the `extract_body()` function iterates through the child nodes and dispatches them to the VProc registers based on their **tags**:
        * **`Operator`**: Becomes the new Operator (after being resolved via `eval_expr`).
        * **`OList`**: Becomes the new OList (after its elements are resolved).
        * **`LCont`**: Becomes the new Local Continuation.
        * **`SInfo`**: (Optional) Updates the source information for debugging/tracing.
    * The VProc state is then updated with these newly resolved registers, completing the transition to the next state.

* **Case 2: Standard/Partial Application (Parameters Present, OList Not Empty)**
    If parameters exist and the `OList` contains at least one argument:
    1.  **Binding**: The first parameter is bound to the first element of the `OList`, creating an extended **VEnv**.
    2.  **Saturation Check**:
        * If all parameters are bound, the closure's `Body` is extracted and executed (as in Case 1).
        * If parameters remain, the **Operator** is updated with a new **Closure** containing the remaining parameters and the extended `VEnv` (Partial Application).

* **Case 3: Argument Waiting (Parameters Present, OList Empty)**
    If the closure requires parameters but the `OList` is empty:
    1.  **Suspension**: The current closure (the Operator) is placed into the **OList**.
    2.  **Resumption**: The interpreter then treats this state as a "Null-like" completion, attempting to resume computation by promoting the **LCont** to the Operator or popping a **CFrame** from the **CChain**.

#### **3. Data Structures and Registers**

* **Operator**: A resolved value. Possible types include:
    * **Closure**: Code paired with a `VEnv`.
    * **SeqClosure**: An internal "argument buffer" closure that holds an `olist` and handles currying/over-application.
    * **PrimFunc / Callable**: Primitive operations.
    * **NULL**: The terminal/resume signal.

* **OList**: A Python `list` serving as the argument buffer. In the ConMa abstract specification, this is referred to as a **Mutable Sequence (MSeq)**, though the current implementation utilizes native Python lists for efficiency.

* **LCont (Local Continuation)**: A pre-evaluated expression or closure representing the "rest of the work" within the current frame.

* **CChain (Continuation Chain)**: A stack of **CFrames**. When a closure is applied to more arguments than it has parameters, the excess arguments are wrapped in a `SeqClosure` and pushed onto the `CChain`.

* **gvenv (Global Environment)**: A Python `dict` mapping Global IDs to their resolved values. This is populated by `load_module()` from the S-expression module file.

* **SeqClosure**: An internal closure type used for currying. It encapsulates:
    * **`olist`**: A buffer of arguments provided during a previous over-application.
    * **`lvenv`**: The **Variable Environment (VEnv)** captured at the time of the SeqClosure's creation, ensuring that the function's lexical scope remains intact during partial application.

#### **4. Evaluation and Transition Logic (`eval_expr`)**
The "Pre-step" resolution logic (which clarifies the confusion in the previous version) is as follows:
* **Variable Resolution**: Local variables are resolved by De Bruijn index from the `VEnv`; Global variables are resolved by ID from `gvenv`.
* **FuncExp Capture**: Function expressions are transformed into **Closures** by capturing the current `VEnv` and `gvenv`.
* **Literal Promotion**: Strings and numbers are treated as terminal values.
* **Strings produced at run time**: A String is kept in source form (quoted and escaped), like a String literal node. Primitives that make a String from Python text (read data, error messages, `__Number_toString__`, ...) use `string_value()`, which always quotes and escapes, so backslashes and quotes in the text are kept.
* **Standard Streams**: `__OS_get_stdin__`, `__OS_get_stdout__` and `__OS_get_stderr__` return `Stream` objects for fds 0, 1 and 2, made on first use and kept in `_STD_STREAMS`, with `std` set. `Stream.close()` only marks such a Stream closed and leaves the fd open, because `print` and the error messages of the interpreter still use it. `__OS_write__` to such a Stream first flushes `sys.stdout` and `sys.stderr`, so its output stays in order with `__OS_print__`.
* **Files and the current directory**: `__OS_open__` opens a file with `os.open` (`"r"`, `"w"` with `O_CREAT | O_TRUNC`, `"a"` with `O_CREAT | O_APPEND`, permission `0o666` before the umask) and wraps the fd in a `Stream`. A directory opened with `"r"` is closed again and reported as `Is a directory`, because `os.open` itself accepts it. `__OS_chdir__` calls `os.chdir`, so the change applies to the whole interpreter: every VProc, `load_program()` and `subprocess.Popen`.
* **Waiting for the OS**: `__OS_wait__` and `__OS_wait_readable__` return to the LCont first (`_prim_return`) and then call `Scheduler.wait`, which marks the VProc `waiting` and records the OSProc or Stream in `Scheduler.waiting`. For an OSProc the exit code is put at the front of the OList when the VProc wakes up. `Scheduler.check_waiting` runs before each step while some VProc waits: with a ready VProc it checks with `select` and a zero timeout; without one it sleeps in `select`. To wake up when a child exits, the first `__OS_wait__` installs a SIGCHLD handler and `signal.set_wakeup_fd`, so SIGCHLD writes a byte to a pipe that is part of the `select`.
* **String primitives**: `__String_length__`, `__String_substring__`, `__String_concat__` and `__String_equal__` first turn their String arguments into Python text with escapes processed (`_string_args`, through `_value_to_display`), so `\n` is one character and positions count code points. Results are made with `string_value()` again, so a String literal and a String made at run time are handled the same way.

#### **5. Module and Environment Management**
* **`gvenv`**: A dictionary of global definitions. Each program has its own `gvenv`: the program run by `vproc_run.py` and each program loaded by `__VProc_spawn__` (`load_program()`, which runs `bin/includer` and `bin/resolver`).
* **Closure `gvenv`**: Each `Closure` keeps the `gvenv` of the module it was created in (`load_module`, `eval_expr`, `extract_body`, `sexp_to_value` and partial application pass it on). A closure passed from one VProc to another therefore keeps referring to the globals of its own program.
* **VProc `gvenv`**: Each VProc keeps the `gvenv` of the program it was started with in `vp["gvenv"]`; the scheduler passes it to `step`.
* **`gvenv` Initialization**: This mechanism allows the ConMa environment to be bootstrapped with essential built-in operations (like `__is_null__` or `__NULL__`) before the first step is executed.
* **`load_module`**: Parses the `(Module ...)` S-expression and populates `gvenv` with Global IDs and their corresponding ASTs or initial values.
* **`PRIMITIVES` Registry**: This is a static mapping of strings to Python functions, used exclusively by `load_module()` to bootstrap the global environment (`gvenv`).
    * When a module S-expression contains a Global ID marked as `Undefined`, `load_module()` checks this registry.
    * If a match is found, the **actual Python function** (e.g., `prim_is_null`) is bound directly to that Global ID in the `gvenv`.
* **S-expression Based Primitives**: If a Global ID is bound to a `(PrimFunc ...)` list in the module file, it is stored in `gvenv` as a list. These are resolved later during the `step` dispatch via `globals()`.

### **Key Implementation Details (Correction Summary)**

| Component | Logic in `vproc_step.py` |
| :--- | :--- |
| **Global Storage** | **`gvenv`** (dictionary), initialized by `load_module`. |
| **Operator Types** | **Closure**, **SeqClosure**, **PrimFunc**, **Callable**, and **NULL**. |
| **SeqClosure Logic** | **Restores `VEnv` and `OList`** from internal buffer. Promotes `OList[0]` to `Operator`, or pops `CChain` if empty. |
| **Body Extraction** | **Tag-based dispatch** (`Operator`, `OList`, `LCont`, `SInfo`) via `extract_body`. |
| **Primitive Path** | **Direct calls** for Python functions; **`globals().get()`** for `PrimFunc` tags. |

#### **6. Running VProcs: `vproc_run.py` and the Scheduler**

`vproc_run.py [--trace] <module_file> <func_name>` loads a module produced by `test/resolver.sh` and starts the first VProc by applying the global function `func_name` to no arguments.
`vproc_run.py [--trace] --main <module_file> [<arg> ...]` starts it the normal way instead: it applies `__start__` (`include/util.se`) to a `SeqClosure` of the `arg` Strings, and `__start__` calls `__main__` with Sinks and a Seq made from the standard Streams.
It then runs the round-robin scheduler (`SCHEDULER`, a `Scheduler` in `vproc_step.py`) until every VProc has finished. `__VProc_spawn__` adds more VProcs to the same scheduler.
See "Scheduling and Termination" in DESIGN.md for the states of a VProc and for when the interpreter exits.

* **VProc value**: A VProc is a `VProc` object (a `dict` of its registers with `serial`, `state` and `gvenv`). `__VProc_current__` and `__VProc_spawn__` pass this object itself as a ConMa value, written as `(__VProcRef__ "N")`.
* **Errors**: A `VProcError` raised by a step finishes only that VProc and prints `Error in VProc N at step S: message` to standard error.
* **Exit codes**:

| Code | Meaning |
|---|---|
| 0 | Every VProc has finished (some may have finished with an error), or `__OS_exit_normal__` was run. |
| 1 | `__OS_exit_error__` was run, or the module or function could not be loaded. |
| 2 | Deadlock: no VProc is ready but some are blocked. Also a wrong command line (reported by `argparse`, for example a missing `func_name`). |

* **Trace** (`--trace`): Before the first step, `[initial]` and the state of the first VProc are written to standard error. After each step, `[step S]` and the state of the VProc that ran it; for any VProc other than the first, the label is `[step S vproc N]`. At the end, `[done] normal termination after S step(s).`

