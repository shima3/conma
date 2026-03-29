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

* **Case 1: Application of a Parameterless Closure (Zero-arity)**
    If the closure's `Head` is empty (no parameters):
    1.  **LCont Preservation**: If the current **LCont** is present, it is pushed onto the **CChain** as a new **CFrame**.
    2.  **Over-argument Handling**: If the current **OList** is not empty, its contents are wrapped into a **SeqClosure** (capturing the current `VEnv`) and pushed onto the **CChain**.
    3.  **Body Execution**: The closure's `Body` is extracted and its elements are dispatched to the VProc registers (`Operator`, `OList`, `LCont`, `SInfo`).

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
* **FuncExp Capture**: Function expressions are transformed into **Closures** by capturing the current `VEnv`.
* **Literal Promotion**: Strings and numbers are treated as terminal values.

#### **5. Module and Environment Management**
* **`gvenv`**: The single source of truth for global definitions. There is no `global_registry` object; the global state is entirely contained within the `gvenv` dictionary passed to the `step` function.
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
