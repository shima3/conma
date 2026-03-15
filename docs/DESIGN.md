# Design of interpreter the ConMa programming language

## Frontend

### Why comments are preserved by the lexer

The `lexer` emits all tokens including comment-related tokens (`LINE_COMMENT_BEGIN`, `LINE_COMMENT_CONTENT`, `BLOCK_COMMENT_BEGIN`, `BLOCK_COMMENT_TEXT`, `BLOCK_COMMENT_END`, `SEXP_COMMENT_BEGIN`) and `NEWLINE`. Comments are not silently discarded during lexical analysis.

This separation ensures that each component has a single, well-defined responsibility:

- The lexer is responsible for segmenting the source text into tokens.
- The comment remover is responsible for deciding which tokens the parser should see.

It also allows the raw token stream (including comments) to be used independently — for example, by documentation generators or syntax highlighters — without modifying the lexer.

### Why NEWLINE tokens are removed by the comment remover, not the lexer

`NEWLINE` tokens serve a purpose within the raw token stream: they mark the boundary between a `LINE_COMMENT_CONTENT` token and the subsequent code, allowing the comment remover to correctly identify where a line comment ends. Once comment removal is complete, `NEWLINE` tokens carry no further meaning for the parser, since ConMa's grammar does not use newlines as statement terminators.

Removing `NEWLINE` in the lexer would conflate two distinct decisions — "what is a token?" and "what does the parser need?" — into a single component. Removing them in the comment remover keeps each stage's scope narrow and explicit.

### Pipeline

```
lexer  →  comment_remover  →  parser
```

The ConMa frontend consists of a driver script, `includer`, which manages the overall transformation from multiple source files to a single AST stream.

1. **`includer`**: Manages the file list using a Breadth-First Search (BFS) strategy. It resolves `Includer` nodes and avoids infinite loops by normalizing paths with `realpath`.
2. **`lexer`**: Tokenizes raw text, preserving comments and newlines.
3. **`comment_remover`**: Filters out comments and newlines to produce a clean token stream.
4. **`parser`**: Generates the AST. When invoked by `includer`, it uses the `--file` flag to attach the filename to the `Program` node and perform automatic `SInfo` insertion.

## AST Metadata

As specified in `AST.md`, location information (`@line:col`) is attached to every node. For multi-file support, the `Program` node is uniquely appended with the quoted filename to serve as a delimiter in the unified output stream.

*Distinction between Meta-Location and SInfo Data*: the coordinates in the `@line:col` suffix are numeric, whereas the line and column values embedded inside a `SInfo` node value are String literals enclosed in double quotes, in accordance with the grammar `SInfo = "(", "__SInfo__", { String }, ")"`.

## Module Loading and GVEnv Representation

The interpreter initiates execution by passing a specified source filename to the includer. The includer reads the corresponding source files (including transitively included files) and outputs their ASTs.

The interpreter then processes these ASTs and converts them into internal module data structures suitable for execution.

A VProc maintains these module structures as its Global Variable Environment (GVEnv).

For debugging purposes, when a VProc is represented as an S-expression, the GVEnv is denoted by the filenames stored in the modules' F nodes, rather than by the internal module structures themselves.

## VProc Environment S-expression Format

The following definitions describe the S-expression format used to display the environment of a VProc (Virtual Process) for debugging purposes.
This format represents the runtime state in a readable form and does not specify the internal data structures used by the implementation.

A VEnv (Variable Environment) consists of two components:
a Local Variable Environment (LVEnv) and a Global Variable Environment (GVEnv).

LVEnv:
A list of bindings. Each binding consists of a variable name and its associated value. The bindings are ordered by their de Bruijn indices: the first element corresponds to index 0 (the innermost lexical binding), the second to index 1, and so forth.

GVEnv:
A list of module filenames that collectively define the global environment accessible to the process.

## Runtime Semantics and Execution

This section describes the dynamic behavior of the VProc during code evaluation, specifically focusing on how closures are applied and how the control flow is managed through the Continuation Chain (CChain).

### **Closure Structure and Operator Application**

A **Closure** consists of three components: **Head**, **Body**, and **LVEnv**.

When a **VProc** executes an **Operator Application** where the **Operator** is a **Closure**, the **VProc** first checks whether the Head of the Operator Closure is empty.

---

#### **Execution When the Head Is Empty**

If the **Head of the Operator Closure is empty**, the **VProc** updates its state as follows without performing any operand binding. Steps 1 and 2 must be completed before Step 3, because they depend on the VProc's current LVEnv, which is replaced in Step 3.

**1. LCont Handling**

If the **LCont** of the **VProc** is not empty, it is pushed onto the **CChain** as a new **CFrame**. If it is empty, no push occurs.

**2. OList Handling**

If the **OList** is not empty, the operands `a_1, a_2, ...` are **surplus operands** — they cannot be bound to any parameter because the Head is empty. To defer their application, a **Seq Closure** is constructed and pushed directly onto the **CChain**.

The Seq is represented by the following S-expression:
```
(,(__Sink__) (__SInfo__ ...) __Sink__ a_1 a_2 ...)
```
This S-expression itself does not contain an LVEnv and therefore is not a Closure; it represents only a Seq at the syntax level. A Seq Closure is formed when this Seq is combined with an LVEnv.

- `a_1, a_2, ...` remain as unevaluated expressions within this S-expression.
- The Seq is converted into a **Closure** using the **VProc's current LVEnv** as its environment.
- The Seq Closure is a form of LCont. Normally it would first be set as the VProc's LCont and then moved to the CChain, but this intermediate step is skipped and it is pushed directly onto the CChain.
- When **`__Sink__`** is later passed to this Closure, its **LVEnv** is restored into the VProc, ensuring that `a_1, a_2, ...` are evaluated in the same environment as when the Seq was created.
- If the VProc has a non-empty **SInfo**, it is inserted into the `(__SInfo__ ...)` component; otherwise, this component is omitted.

If the **OList** is empty, nothing is pushed.

**3. State Update (Body and LVEnv)**

- The **SInfo**, **Operator**, **OList**, and **LCont** of the **Operator Closure's Body** are loaded into the **VProc** as its current state.
- The **LVEnv** of the **Operator Closure** becomes the **VProc's current LVEnv**.

---

#### **Partial Application When the Head Is Not Empty**

If the **Head of the Operator Closure is not empty**, two sub-cases apply depending on whether the **OList** is empty.

**When the OList is not empty**, the VProc creates a new Closure derived from the Operator Closure by binding operands from the OList to parameters in the Head of the Operator Closure in order until either the OList or the Head is exhausted. The Head of the new Closure consists of the parameters that have not yet been bound. Each operand is processed as follows:

- **If the operand is a Function**: A new Closure is constructed from the Function's Head and Body using the VProc's current LVEnv. This Closure is assigned to the parameter.
- **If the operand is not a Function**: The operand value is assigned directly to the parameter.

The resulting Closure is set as the VProc's new **Operator**.
The VProc's LCont remains unchanged.
If the OList contains more operands than the Head has parameters, the VProc's OList is set to the remaining operands, and the new Closure's Head is empty.
The next Operator Application will then handle these surplus operands via the Seq Closure mechanism described above.
If the OList contains no more operands than the Head has parameters, the VProc's OList is set to empty.

**When the OList is empty**, the application cannot proceed yet; the result is a partial application. The VProc updates its state as follows:

1. The current **Operator** (the not-yet-fully-applied Closure) is appended to the **OList** as an additional operand.
2. The current **LCont** is set as the VProc's new **Operator**.
3. The **LCont** is cleared (set to empty).

This effectively passes the partially applied Closure to the LCont as its argument, allowing the LCont to provide the missing operand(s) when invoked.
---

#### Examples of VProc State Transitions

The following examples illustrate the state transitions of the VProc
during one Operator Application.
These examples are informative and do not introduce additional rules.

1. Execution (Head is empty)
This example shows the state transition of the VProc when a saturated Closure (whose Head is empty) is applied to additional operands.
Before Application
* Operator: `(Closure (Head: ∅) (Body: <Expr>) (LVEnv: Env_A))`
* OList: `(a1 a2 a3)`
* LCont: `(LCont_X)`
* SInfo: `("1" "5")` (strings used as internal data)
* LVEnv: Env_Current (the VProc's current local environment, used to form the Seq Closure)
After Application
1. CChain Update: `LCont_X` is pushed onto the CChain, and a Seq Closure is then pushed directly on top of it.
   * Seq Closure: A pair consisting of the following Seq S-expression and the VProc's current LVEnv.
   * Seq (S-expression): `(,(__Sink__) (__SInfo__ "1" "5") __Sink__ a1 a2 a3)`
2. VProc State: The contents of the Body of the Operator Closure are loaded into the VProc.
   * Operator: `<Expr>` (the Operator contained in the Body)
   * OList: (the OList contained in the Body)
   * LCont: (the LCont contained in the Body)
   * LVEnv: `Env_A` (the environment stored in the Closure)
2. Partial Application (Head is not empty)
This example shows the creation of a derived Closure when the provided operands are insufficient to bind all parameters.
Before Application
* Operator: `(Closure (Head: (p1 p2)) (Body: <Expr>) (LVEnv: Env_A))`
* OList: `(a1)`
* LCont: `(LCont_X)`
After Application
1. Operator: A new derived Closure is created and set as the Operator.
   * Closure: `(Closure (Head: (p2)) (Body: <Expr>) (LVEnv: Env_A + {p1: a1}))`
2. OList: `()` (empty, because `a1` has been consumed by parameter binding).
3. LCont: `(LCont_X)` (unchanged).
3. Partial Application (OList is empty)
This example shows how a partially applied Closure is passed to the current continuation when no operands are available.
Before Application
* Operator: `(Closure (Head: (p2)) (Body: <Expr>) (LVEnv: Env_B))`
* OList: `()`
* LCont: `(LCont_Y)`
After Application
1. Operator: `LCont_Y` (the current continuation becomes the next Operator).
2. OList: `((Closure (Head: (p2)) ...))` (the original partially applied Closure becomes the single operand).
3. LCont: `∅` (empty).

### CChain Exhaustion and Null Passing

If the `__CChain_pop_CFrame__` primitive is invoked when the **Continuation Chain (CChain)** is empty, the following behavior is guaranteed:

1. **Null Passing**: The interpreter passes a `null` value as the argument to the current **Local Continuation (LCont)**.
2. **Execution Resumption**: The `LCont` is then executed with this `null` value, allowing the program to handle the end of the chain or an empty state through standard null-check logic (e.g., using `__is_null__`).

**Operational Note:**
This design ensures that a VP (Virtual Process) does not abruptly terminate or enter an undefined state when the chain is exhausted. Instead, it provides a predictable signal (`null`) to the functional layer, maintaining the language's emphasis on explicit control flow.
