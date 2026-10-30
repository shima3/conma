# Specification of the ConMa programming language

The ConMa (abbreviation for Continuation Machine) programming language is based on lambda calculus where all functions are curried in continuation passing style (CPS).
This programming language aims for a concise specification for portability.
It is intended to be used as an intermediate language that translates from more beginner-friendly languages ​​to this language and executes them.
A certain degree of readability is necessary for debugging, but syntactic sugar is not required.

This page describes a specification of the language.
For the meaning of terms, see [TERMS.md].
For the primitives, see [PRIMITIVES.md].

## EBNF Syntax Definition

This document separates the **lexical definitions** (used for tokenization) and **syntactic definitions** (used for parsing).
For the EBNF notation used, see [EBNF.md].

### Lexical Definitions (Tokens)

```ebnf
Token        = Significant | Whitespace ;
Significant  = Symbol | String | "(" | ")" | "," | LineCommentBegin | BlockCommentBegin | BlockCommentEnd | SExpCommentBegin ;

Symbol      = SymbolChar, { SymbolChar } ;
SymbolChar  = Letter | Digit | SpecialChar ;
Letter      = "a"..."z" | "A"..."Z" | "_" ;
Digit       = "0"..."9" ;
SpecialChar = "!" | "?" | "*" | "+" | "-" | "/" | "=" | "<" | ">" | "$" | "%" | "&" | "|" | "." | "^";

String      = '"', { StringCharacter }, '"' ;
StringCharacter = Character | EscapeSequence ;
Character   = ? any valid character except '"', '\', and Newline ? ;
EscapeSequence  = "\", ( '"' | "\" | "n" | "t" | "r" ) ;

Whitespace  = " " | "\t" | Newline ;
Newline     = "\n" | "\r\n" ;

LineComment = LineCommentBegin, { LineCommentCharacter } ;
LineCommentBegin = ";" ;
LineCommentCharacter = ? any valid character except Newline ? ;

BlockComment = BlockCommentBegin, { BlockCommentContent }, BlockCommentEnd ;
BlockCommentBegin = "#|" ;
BlockCommentEnd = "|#" ;
BlockCommentContent = BlockComment | BlockCommentText ;
BlockCommentText    = ? character sequence not containing #| or |# ? ;

SExpCommentBegin = "#;" ;
```

Comments are removed after lexical analysis and before syntactic analysis.

* LineComment (token sequence):
  LineCommentBegin { any token except NEWLINE }

* BlockComment (token sequence):
  BlockCommentBegin { BlockComment | any token except BlockCommentBegin or BlockCommentEnd } BlockCommentEnd

* S-Expression Comment
An S-expression comment begins with "#;" and discards the following S-expression.
The removed S-expression may be an atom or a list.

### Syntactic Definitions (Grammar)

```ebnf
Program    = { Statement } ;
Statement  = Includer | Definition ;
Includer   = "(", "include", String, ")" ;
Definition = "(", "define", Variable, Function, ")" ;
Variable   = Symbol - Hat ;
Function   = Hat, Head, Body ;
Hat        = "," ;
Head       = "(", { Parameter }, ")" | Parameter ;
Parameter  = Variable ;
Body       = [ SInfo ], [ Operator, OList ], [ LCont ] ;
Operator   = Variable | FuncExp ;
OList      = { Operand } ;
LCont      = Function ;
Operand    = Expression ;
Expression = Variable | String | FuncExp ;
FuncExp    = "(", Function, ")" ;
SInfo      = "(", "__SInfo__", { String }, ")" ;
```

In `Body`, the first `Expression` is the **Operator**, and the second and subsequent `Expression`s form the **OList**.

A `Head` that is a single `Parameter` without parentheses makes the `Function` **variadic**: `, args Body` (see "Variadic Functions" below).

**LCont Rules:**
- Any `Function` intended to be an `Operand` must be wrapped in parentheses as a `FuncExp`.
- An unparenthesized `,` symbol signals the end of the `OList` and the start of the `LCont`.

In the first version of the language:

* Top-level expression evaluation is not allowed. Only `Statement`s are permitted at the top level.
* Numeric literals are not supported.

**Omission Rules and Interpretation of `Body`**

Within a `Body`, the `Operator` and `OList` components **may be omitted only together**.

* **Semantics of Omission**:
  If both `Operator` and `OList` are omitted, the interpreter **treats the `Operator` as `Null`** (i.e., no operation) and **the `OList` as an empty list**.

* **Syntactic Constraint (Disambiguation)**:
  It is **not permitted to omit the `Operator` while specifying the `OList` alone**.

  *Reason*:
  If this were allowed, the first element of the `OList` could not be unambiguously distinguished from the `Operator` during parsing.

* **Valid Forms**:

  1. `[ SInfo ] Operator OList LCont`
     (fully specified form)

  2. `[ SInfo ] LCont`
     (`Operator` and `OList` are both omitted; `Operator` is interpreted as `Null` and `OList` as empty)


## Static Semantics

SInfo (Source Info) is information for debugging purposes and does not affect basic operation. It is used to indicate the location in the source code when an error occurs or some debuggers are running the program.

The Includer (include FileName) includes the Program from the file named FileName at loading time.
ConMa source files are expected to have the `.se` extension.

* `include "filename"`:
    The interpreter recursively scans and loads the specified file.
    If "filename" is provided without an extension, the interpreter searches for "filename.se".

The Definition (define Variable Function) binds the identifier Variable to the closure of Function, thereby defining a new named function in the `VEnv`.

Recursive definitions are permitted using `define`.
Example:
```
(define f ,(x) x (f x))
```

Shadowing is permitted.
Example:
```
,(x) (,(x) x)
```
is semantically equivalent to:
```
,(x) (,(x2) x2)
```

---

## Evaluation Semantics

The ConMa interpreter employs a strict **Call-by-Value** strategy for `Variable` and `String` types, and a **Call-by-Name (Thunk-based)** strategy for `FuncExp`.

### Argument Processing (Eager vs. Lazy)

Before a function application occurs, each `Operand` in the `OList` is processed into a **Value** based on its syntactic category:
A Value can be a String, Closure, Ref, CFrame, CChain, VirtualProcess, Sequence, Sink, null, etc.

* **Variable**: Evaluated immediately. The identifier is replaced by the value currently bound to it in the `VEnv`.
* *Note on `Ref`:* If a variable is bound to a `Ref` (reference cell), the `Ref` itself is passed as the value. The content inside the `Ref` is **not** dereferenced during this stage.

* **String**: Treated as a literal value and passed as-is.
* **FuncExp**: Not executed. It is captured along with the current `VEnv` to create a **Closure** (Thunk). This closure is passed as the value, and its internal `Body` is only evaluated if/when explicitly invoked by the callee.

### Evaluation Order and Side Effects

Because `Variable` and `FuncExp` are resolved into values (either raw data or closures) before the function body is entered, the execution of the function is **order-independent** regarding its arguments. No side effects occur during the argument resolution phase itself, as dereferencing or state mutation only happens via explicit internal primitives (e.g., `__Ref_get__`, `__Ref_set__`).

### Application Process

Once the `Operator` and processed `OList` are ready:

1. **Operator Resolution**: The `Operator` is resolved to a function or closure.
2. **Binding**: The processed values from `OList` are bound to the `Parameter` identifiers in the function's `Head`.
3. **LCont Handling**: If a `LCont` (Local Continuation) is present, a `CFrame` containing the `LCont`'s closure, current `SInfo`, and next `CFrame` is pushed onto the `CChain`.
4. **Body Execution**: The `Body` of the function is evaluated within the new environment.

Example:

```
(,(p1 p2 p3 ...) B) A1 "A2" (,() A3)
```

Evaluation proceeds as follows:

* The value bound to `A1` is assigned to `p1`.
* The string `"A2"` is assigned to `p2`.
* The closure of `(,() A3)` is assigned to `p3`.

After these assignments, `B` is evaluated.

### Variadic Functions

A `Function` whose `Head` is a single `Parameter` without parentheses, `, args Body`, is **variadic**.

* When it is applied, all the values of the `OList` of that application are put into one `Sequence` (a `Seq`, see "Sequence" below), which is bound to the `Parameter`. With no values, an empty `Seq` is bound.
* A variadic function in the Operator position is always executed, even without values. (A function with parameters that is applied to no values is not executed but passed on as a value; see DESIGN.md.) To pass a variadic function without executing it, use it as an `Operand`, or pass it with `__NULL__`: `__NULL__ (, args ...) ,(f)`.
* It takes the values of one application only, so there are never surplus operands. In `(, args B) a1 a2 ,(sink) sink a3 a4`, `args` is the `Seq` of `a1 a2`; `a3 a4` are applied to what the function returns.
* A `Seq` passes its elements to a `Sink` grouped in an implementation-defined way. A variadic function used as the `Sink` of a `Seq` may therefore receive the elements in several applications.
* `args` is a single variable: for the de Bruijn index it counts as one parameter.

Example:

```
(define show , args
  args printSink)

show ,()             ; args is an empty Seq
show "a" "b" ,()     ; args is (,(sink) sink "a" "b")
pair "x" "y" , values ...   ; as an LCont: values receives all the values returned by pair
```

---

## Runtime Model

A process managed by the operating system is called an **OS Process**.
A process managed by the interpreter is called a **Virtual Process (VProc)**.

In the first version:
A **Virtual Process (VProc)** is scheduled using round-robin scheduling.
The scheduling granularity is **one Operator application**.

A single scheduling step is defined as follows.

1. The scheduler selects the next runnable VProc from the ready queue.
2. The selected VProc executes exactly **one Operator application**.
3. After the Operator application completes, control returns to the scheduler.
4. If the VProc is still runnable, it is placed at the end of the ready queue.
5. The scheduler then selects the next VProc in the queue.

An **Operator application** is defined as the evaluation of one `Body`:
```
[ SInfo ], Operator, OList, [ LCont ]
```

The execution of an Operator application consists of:

1. Determining the function designated by `Operator`.
2. Assigning the values of `OList` to the corresponding parameters.
3. Creating and pushing a CFrame if required by the CPS rules.
4. Beginning execution of the function body.

An Operator application is considered complete immediately before the next Operator to be evaluated becomes the current Operator.
Invoking LCont is considered part of the following Operator application.
Internal primitives are executed as one Operator application.

At that point, the scheduler may switch execution to another VProc.
If the executing VProc becomes blocked (for example, __VProc_suspend__), waits for the OS (__OS_wait__, __OS_wait_readable__) waits for another VProc to finish (__VProc_wait__), waits for another VProc to evaluate a LazyList (__LazyList_claim__) or waits for a Ref to change (__Ref_wait_change__), it is removed from the ready queue until it becomes runnable again. While no VProc is ready, the interpreter sleeps until a VProc that waits for the OS can continue.

Termination:
* A VProc finishes when its Operator is null, its LCont is absent, and its CChain is empty.
* If an error occurs while a VProc runs (for example, an unbound variable or a primitive applied to a value of the wrong type), only that VProc finishes. An error message with the VProc's number is written to standard error, and the other VProcs keep running.
* The interpreter exits when every VProc has finished. Its exit code is 0, whether or not some VProcs finished with an error.
* `__OS_exit_normal__` and `__OS_exit_error__` exit the whole interpreter at once, with exit code 0 and 1 respectively, even if other VProcs are still running.
* If no VProc is ready or waiting for the OS but some are blocked or waiting for another VProc, the interpreter reports a deadlock and exits with exit code 2.

The function designated as the `Operator` in `Body` is applied to `OList`.
A CFrame consisting of `SInfo` and the closure of `LCont` is pushed onto the CChain.

In normal mode:
* If there is no LCont, no CFrame is pushed.
* Therefore, tail recursion does not consume CChain space.

In debug mode:
* Even if there is no LCont, a CFrame consisting of `SInfo` and a closure of __nop__ is pushed.
* __nop__ performs no operation.
* This CFrame exists solely to preserve `SInfo` for stack trace generation.

---

## Data Model

An **output sink** and an **error sink** are of type `Sink`.
A `Sink` is a function that accepts an argument and returns a `Sink`.

An **argument sequence** is of type `Seq`. The **input sequence** that `__main__` takes is a `LazyList` of lines (see "Main"); the name inputSeq is kept for the parameter of `__VProc_spawn__`.
A `Seq` is a function that accepts one `Sink`.
It applies that `Sink` to zero or more elements and returns the final `Sink` produced by those applications.

A `Seq` applies its elements to a `Sink` in order.
The grouping of arguments in each call is implementation-defined, provided that the observable result is equivalent to successive single-argument applications.

Example:
(define sample_sink ,(Value)
  __print__ Value ,()
  sample_sink)

(define sample_seq ,(sink)
  sink "A" "B" "C" ,(last_sink)
  last_sink)

---

## **OS-Related Data Types**
* **Stream**: A handle for I/O operations (File, Pipe, or Standard I/O).
* **OSProc**: A handle representing an external process, used for synchronization or signaling (e.g., waiting for termination).

---

# Standard Definition

### `Naming conventions`
Names of the form `__name__` are reserved for built-in primitives and standard definitions.
In ConMa, constants are also a kind of function: each of them can be applied as an operator.
The case of a name follows how the thing it names would be treated in a non-functional language: a name that would be a constant there is written in upper case, and a name that would be a function there is written in lower case.
They follow these rules:
* **Constants** are written in upper case: `__NULL__`, `__TRUE__`, `__FALSE__`.
  A constant names one specific value of a data type, such as null or a Boolean.
  For example, `__TRUE__` is a closure `,(then else) ...` and is applied to select a branch, but it is a constant because it stands for the Boolean value true.
* **Functions** are written in lower case: `__nop__`, `__not__`, `__if_then_else__`.
  A function names an operation.
  It is still a function when it is passed as a value, such as `__nop__` passed to `__if__` or `__print_exit_error__` passed as `onError`.
* When a function name starts with a type name, the type name keeps its own spelling and the operation part is in lower case: `__CChain_get__`, `__OS_print__`, `__PDict_lookup_or__`.
* A type name inside the operation part also keeps its own spelling, joined to the neighboring word without `_`: `__Number_fromString__`, `__Number_toString__`, `__OS_get_envDict__` (an env `Dict`). After `_`, it is written as its own word: `__OS_get_String__`.

---

### `No operation`
Behavior:
Do nothing.
Invokes the current LCont with no argument
It is defined as follow:
```
(define __nop__ ,())
```
---

### `Continuation`
```
(define __Cont_get__ ,()
  __CChain_pop_LCont__ ,(lc)
  __CChain_get__ ,(cc)
  lc (, args
    __if_then_else__ (,() __is_null__ cc)
      (,() __CChain_set__ cc)
      (,()
        __CChain_set__ cc ,()
        __CChain_pop_LCont__ ,(lc)
        args lc)))
```
__Cont_get__ passes a continuation to the LCont. If you get it at the beginning of a function, you can use it as return.
The continuation is a variadic function: it sets the CChain back to that of the caller of the function and passes all its arguments to the caller's LCont. Without arguments, it returns to the caller without values. If the function has no caller (the CChain is empty), the continuation empties the CChain, so the VProc finishes.
The continuation must be variadic: if it took its arguments as surplus operands, they would be kept on the CChain and lost when the CChain is set back.

Example:

(define f ,()
  __Cont_get__ ,(return)
  g ,(x y)
  stdout x ", " y ,(out)
  return)

(define g ,()
  __Cont_get__ ,(return)
  return "12" "34")

---

### `Sequence`
```
(define __Seq_push__ ,(First Rest Sink)
  Sink First ,(sink)
  Rest sink)

(define __Seq_get__ ,(onEmpty Seq)
  __Cont_get__ ,(return)
  __CChain_get__ ,(outer)
  __LList_length__ outer ,(outerLen)
  Seq (,(first)
        __CChain_get__ ,(inner)
        __LList_length__ inner ,(innerLen)
        __Number_subtract__ innerLen outerLen ,(n)
        __Number_fromString__ __print_exit_error__ "1" ,(one)
        __Number_subtract__ n one ,(n)
        __Seq_take_CFrames__ n inner __NULL__ ,(frames)
        return first (,(sink)
          __CChain_get__ ,(chain)
          __Seq_push_CFrames__ frames chain ,(chain)
          __CChain_set__ chain ,()
          __NULL__ sink)) ,(sink)
  onEmpty)

(define __Seq_get_all__ ,(Seq)
  __CChain_pop_LCont__ ,(lc)
  Seq lc ,(dummy)
  __exit__ "__Seq_get_all__ insufficient number of elements")

(define __Seq_print__ ,(seq)
  __Cont_get__ ,(return)
  __Seq_get__ return seq ,(str rest)
  __OS_print__ str ,()
  __Seq_print__ rest)

(define __println__ , args
  __Seq_print__ args ,()
  __OS_print__ "\n")
```

`__Seq_get__ onEmpty Seq ,(first rest)` returns the first element of `Seq` and a `Seq` of the rest. If `Seq` is empty, it applies `onEmpty` instead, with the continuation of `__Seq_get__`.
When the `Sink` gets the first element, the rest of the `Seq` is in the CFrames above the LCont of the application of `Seq`, `,(sink) ...`: the Seq Closure of the surplus operands, if the `Seq` passes several elements in one application; the LCont of the application of the `Sink`, if it passes them one at a time; and below them, the LConts of functions that the `Seq` called with an LCont, if it is made of other `Seq`s. `__Seq_take_CFrames__` takes these CFrames, counted from the lengths of the CChain before applying `Seq` and inside the `Sink` (`__LList_length__`). `rest` puts them back on top of the CChain of its caller (`__Seq_push_CFrames__`) and returns its `Sink` to them, so the `Seq` goes on with the new `Sink` and finally returns to the caller of `rest`. `rest` can be applied more than once.
After the last element there are no such CFrames, so `rest` returns its `Sink` at once: it is an empty `Seq`. When `Seq` is empty, the `Sink` is not applied and `Seq` returns it to `,(sink) ...`, which applies `onEmpty`.

`__Seq_print__ Seq` writes the elements of `Seq` with `__OS_print__`, one after another. `__println__` is variadic: it writes all its arguments and then a newline.

`__Seq_get__` (with `__Seq_take_CFrames__`, `__Seq_push_CFrames__` and `__LList_length__`), `__Seq_print__` and `__println__` are defined in `include/util.se`; `__Seq_push__` and `__Seq_get_all__` are not yet.
---

### `LazyList`
```
(define __LazyList_new__ ,(thunk)
  __Ref_new__ thunk)

(define __LazyList_empty__ ,()
  __Ref_new__ __NULL__)

(define __LazyList_cons__ ,(head tail)
  __LList_cons__ head tail ,(cell)
  __Ref_new__ cell)

(define __LazyList_uncons__ ,(onEmpty lazyList)
  __LazyList_force__ lazyList ,(cell)
  __if_then_else__ (,() __is_null__ cell)
    onEmpty
    (,() __LList_uncons__ __print_exit_error__ cell))
```

A `LazyList` is a list whose elements are computed when they are first needed, and only once. Where a `Seq` pushes its elements to a `Sink`, a `LazyList` is taken apart from the front, so getting the first element and the rest needs no CFrames of the CChain, and the list can be walked any number of times without running its computation again.

A `LazyList` is a `Ref`. Its value is one of:
* a Closure: a thunk not yet evaluated. Applied to no values, it returns a `LazyList`.
* null: an evaluated empty list.
* an `LList`: an evaluated cell. Its head is the first element and its tail is the rest, a `LazyList`.
* a `VProc`: a mark that this VProc is evaluating the thunk, or holds a `LazyList` that has no value yet (`__LazyList_hold__`).

* `__LazyList_new__ thunk ,(LazyList)` makes a `LazyList` of a thunk. The thunk is applied when the `LazyList` is first evaluated, and never again.
* `__LazyList_empty__ ,(LazyList)` and `__LazyList_cons__ head tail ,(LazyList)` make evaluated `LazyList`s. `tail` is a `LazyList`.
* `__LazyList_force__ LazyList ,(cell)` evaluates the `LazyList` and returns its cell, null or an `LList`. It gets the thunk with the primitive `__LazyList_claim__`, which puts the current VProc as the mark, applies the thunk, evaluates the `LazyList` that the thunk returns, and sets the cell of that as the value with the primitive `__LazyList_publish__` (memoization). A thunk may return a `LazyList` not yet evaluated, such as `ys` in `append xs ys` when `xs` is empty. If another VProc is evaluating the `LazyList`, `__LazyList_claim__` waits until it has finished, and the cell that it set is returned. The claim is made inside `__LazyList_force_claim__`, which `__LazyList_force__` calls with an LCont, so the CChain at the claim is never empty and stays at the bottom of the CChain while the thunk runs. A `LazyList` that depends on itself (the current VProc is the mark) and one whose evaluating VProc finished without setting the cell stop the interpreter with an error. A `LazyList` may still have itself as its tail, because the tail is not evaluated while the cell is made.
* `__LazyList_uncons__ onEmpty LazyList ,(head tail)` returns the first element and the rest. If the `LazyList` is empty, it applies `onEmpty` instead, with the continuation of `__LazyList_uncons__`, as `__Seq_get__` does.
* `__LazyList_is_empty__ LazyList ,(Boolean)` returns `__TRUE__` if the `LazyList` is empty.
* `__LazyList_fromSeq__ Seq ,(LazyList)` makes a `LazyList` of the elements of a `Seq`. It gets the elements with `__Seq_get__` one at a time, when the `LazyList` is walked. For a `Seq` of the lines of a Stream, the lines are read once, however many times the `LazyList` is walked.
* `__LazyList_toSeq__ LazyList ,(Seq)` makes a `Seq` that passes the elements of the `LazyList` to its `Sink`.
* `__LazyList_lines__ Stream ,(LazyList)` makes a `LazyList` of the lines of an input Stream, each including its newline; a last line without a newline is an element as it is. It reads the Stream directly (not through a `Seq`), only when the `LazyList` is walked, and waits with `__OS_wait_readable__` when no data has come yet.
* `__LazyList_pipe__ capacity onClosed ,(LazyList Sink close cancel)` makes a pipe: the values written to `Sink`, in the order they are written, become the elements of `LazyList`, and `close ,()` ends it.
    * Each element is behind a `LazyList` with a thunk (outer). When a reader evaluates it, the thunk tells the writer that the element is wanted (it sets a state `Ref` to a new value) and returns a `LazyList` that the writer holds with `__LazyList_hold__` (inner). Each value publishes a cell `(value . next outer)` to the inner, and `close` publishes null, so a VProc that walks the `LazyList` waits in `__LazyList_claim__` until the next value or `close`.
    * Capacity: the writer may write at most `capacity` (a Number, 0 or more) elements that no reader has asked for yet. It keeps the outers not yet evaluated in a queue; when the queue has more than `capacity` of them, `Sink` drops those that have been evaluated from its front, and otherwise waits with `__Ref_wait_change__` until a reader asks for an element or cancels. With `capacity` 0, each value is written only when a reader waits for it.
    * Cancel: a reader that needs no more elements applies `cancel ,()` (like a closed pipe that gives SIGPIPE to the writer). The next `Sink` application, or the one that is waiting, ends the `LazyList` as `close` does and applies `onClosed` instead, with the continuation of the `Sink` application. A reader that finishes without `cancel` does not wake the writer.
    * `Sink` and `close` must be used in the VProc that called `__LazyList_pipe__`; in another VProc, or after `close`, `__LazyList_publish__` stops the VProc with an error. `close` after the `LazyList` has ended does nothing. If that VProc finishes without `close`, a VProc that evaluates the tail gets an error.
    * The `Sink`, `close` and `cancel` Closures are made in functions that take only the `Ref`s they use, so that they do not keep the first cell of the `LazyList` (and every element written so far) alive.

These are defined in `include/util.se`, on top of the primitives `__LazyList_claim__`, `__LazyList_hold__`, `__LazyList_publish__` and `__Ref_wait_change__` (see PRIMITIVES.md).
Each of the two primitives reads and sets the value of the `Ref` in one Operator application, so the scheduler cannot switch VProcs between the test and the setting of the mark, and the same `LazyList` may be evaluated by several VProcs: the thunk runs once, in the VProc that claimed it first, and the others wait for its cell.
A thunk may leave `__LazyList_force__` without returning, for example by the `return` of a `__Cont_get__` outside it, or by an `onError` continuation. `__LazyList_claim__` keeps the CChain of the VProc at the claim; while the VProc evaluates the thunk, that CChain is the bottom of its CChain. When it is no longer (the VProc has left the evaluation), the `LazyList` is abandoned: a later claim, in the same VProc or another one, claims it again and gets the thunk again, and VProcs waiting for it wake up and do so. A thunk that has been left may therefore run more than once. If the VProc goes back into the evaluation it left (by a continuation captured inside the thunk) after the `LazyList` has been claimed again, its `__LazyList_publish__` stops it with an error.
---

### `Stream`
```
(define __Seq_stream__ ,(onEmpty Seq)
  __Ref_new__ Seq ,(ref)
  (,(sink)
    __Ref_get__ ref ,(seq)
    __Seq_get__ onEmpty seq ,(first rest)
    __Ref_set__ ref rest ,()
    sink first ,(sink2)
    __Ref_get__ ref ,(rest2)
    rest2 sink2))

(define __Sink_stream__ ,(Sink)
  __Ref_new__ Sink ,(ref)
  (,(value)
    __Ref_get__ ref ,(sink)
    sink value ,(sink2)
    __Ref_set__ ref sink2 ,()
    sink2))
```
---

### `Virtual Process`
```
(define __VProc_wait_until__ ,(cond)
  __VProc_current__ ,(proc)
  __fix__ (,(loop)
    cond ,()
    __VProc_suspend__ proc ,()
    loop))
```
---

### `Ref`
```
(define __Ref_wait_until__ ,(ref cond)
  __VProc_wait_until__ (,()
    __Ref_get__ ref ,(value)
    cond value))

(define __Ref_wait_for_null__ ,(ref)
  __Cont_get__ ,(return)
  __Ref_wait_until__ ref (,(value)
    if (,() __is_null__ value) return))

(define __Ref_wait_for_non_null__ ,(ref)
  __Cont_get__ ,(return)
  __Ref_wait_until__ ref (,(value)
  unless (,() __is_null__ value) (,() return value)))
```

### `Pipe`
```
(define __Pipe_new__ ,()
  __Ref_new__ __NULL__ ,(inProcRef)
  __Ref_new__ __NULL__ ,(outProcRef)
  __Ref_new__ __NULL__ ,(valueRef)
  (,(sink) sink inProcRef outProcRef valueRef))

(define __Pipe_in__ ,(Pipe Sink)
  __VProc_current__ ,(inProc)
  __Seq_get_all__ Pipe ,(inProcRef outProcRef valueRef)
  __Ref_set__ inProcRef inProc ,()
  __Ref_wait_for_non_null__ outProcRef ,(outProc)
  __Ref_get__ valueRef ,(value)
  __Ref_set__ inProcRef __NULL__ ,()
  __VProc_resume__ outProc ,()
  __Ref_wait_for_null__ outProcRef ,()
  Sink value ,(sink)
  __Pipe_in__ Pipe sink)

(define __Pipe_out__ ,(Pipe Value)
  __VProc_current__ ,(outProc)
  __Seq_get_all__ Pipe ,(inProcRef outProcRef valueRef)
  __Ref_set__ valueRef Value ,()
  __Ref_set__ outProcRef outProc ,()
  __Ref_wait_for_non_null__ inProcRef ,(inProc)
  __VProc_resume__ inProc ,()
  __Ref_wait_for_null__ inProcRef ,()
  __Ref_set__ outProcRef __NULL__ ,()
  __Pipe_out__ Pipe)

(define __Pipe_new_in_out__ ,()
  __Cont_get__ ,(return)
  __Pipe_new__ ,(pipe)
  __Pipe_in__ pipe ,(in)
  __Pipe_out__ pipe ,(out)
  return in out)
```

---

### `Main`

```
(define __main__ ,(err args in out)
  __Ref_new__ in ,(position)
  __main_run__ err args position out)

(define __main_run__ ,(err args position out)
  __Cont_get__ ,(exit)
  __PDict_put__ "__EXIT__" exit ,()
  __PDict_put__ "__STD_IN__" position ,()
  __PDict_put__ "__STD_OUT__" out ,()
  __PDict_put__ "__STD_ERR__" err ,()
  main args ,()
  exit "")
```

* err: a Sink used to send values to the standard error stream.
* args: a Sequence of strings provided to the program at startup.
* in: a LazyList of the lines of the standard input stream. `__main__` puts a `Ref` of the read position, whose value is the `LazyList` of the lines not yet read, in `PDict`, and goes on in `__main_run__`, which does not take `in`, so that no continuation keeps the first cell and the lines already read can be collected.

* `stdin ,(LazyList)` returns the `LazyList` of the lines not yet read. It does not move the read position.
* `__stdin_read__ onEmpty ,(line)` returns the next line and moves the read position past it. If no line is left, it applies `onEmpty`, with a continuation inside `__stdin_read__`, as `__LazyList_uncons__` does.
* out: a Sink used to send values to the standard output stream.

---

### `Exit`

```
(define __PDict_lookup_or_error__ ,(key)
  __PDict_lookup_or__ key (,(key)
    __OS_print_error__ "not found key: " ,()
    __OS_print_error__ key ,()
    __OS_print_error__ "\n" ,()
    __OS_exit_error__))

(define __exit__ ,(String)
  __PDict_lookup_or_error__ "__EXIT__" ,(exit)
  exit String)
```

### `Standard input, output, error`

```
(define stdin ,()
  __PDict_lookup_or_error__ "__STD_IN__" ,(position)
  __Ref_get__ position)

(define __stdin_read__ ,(onEmpty)
  __PDict_lookup_or_error__ "__STD_IN__" ,(position)
  __Ref_get__ position ,(lazyList)
  __LazyList_uncons__ onEmpty lazyList ,(line rest)
  __Ref_set__ position rest ,()
  __NULL__ line)

(define stdout ,()
  __PDict_lookup_or_error__ "__STD_OUT__")

(define stderr ,()
  __PDict_lookup_or_error__ "__STD_ERR__")
```
---

### `print`
```
(define __print__ ,(str)
  __CChain_pop_LCont__ ,(lc)
  stdout str "\n" ,(out)
  lc)
```
---

### `Control flow`
```
(define __if_then_else__ ,(Boolean then else)
  Boolean then else ,(clause)
  clause)

(define __if__ ,(Boolean then)
  __if_then_else__ Boolean then __nop__)

(define __unless__ ,(Boolean else)
  __if_then_else__ Boolean __nop__ else)

(define __fix__ ,(x)
  x (,() __fix__ x))

(define __TRUE__ ,(then else)
  __NULL__ then)

(define __FALSE__ ,(then else)
  __NULL__ else)

(define __not__ ,(Boolean)
  Boolean __FALSE__ __TRUE__)
```

# Execution Rules

* If an unbound variable is encountered, an error is displayed and execution terminates.
* If too few arguments are supplied, partial application occurs and this is not treated as an error.
* If too many arguments are supplied, a local continuation that applies the remaining arguments is pushed onto the CChain, and this is not treated as an error.

Example: (,(p1) p1) a1 a2 -> (,(f) f a2) a1
(,(f) f a2) is the local continuation.

* If the Operator is null, the following procedure is performed.
    1. If the LCont is absent:
        * If the CChain is empty, the VProc terminates with an error.
        * Otherwise:
            1. Pop a CFrame from the CChain.
            2. Set the Closure contained in the popped CFrame as the Operator of the VProc.
    2. If the LCont is present:
        1. Set the **LCont** as the **Operator** of the VProc.
        2. Set the **LCont** of the VProc to **absent**.
* If the Operator is a Variable, it replaces with the Value assigned it in `VEnv`.
* If the Operator is a FuncExp, it replaces with the closure of the FuncExp.
* If the Operator is a closure, it applies the closure to the OList.
* If the Operator is not a Variable, a FuncExp, or a closure, an error is displayed and execution terminates.

---

# Program Entry Point: `main`

A ConMa program must define a `main` function.
A program is started by spawning a virtual process (`__VProc_spawn__`) with the entry point `"__main__"`, which applies `__main__` (defined in `include/util.se`) to the following arguments: an errorSink, an argumentSeq, inputSeq, and an outputSink.
The interpreter starts the first VProc with `__start__` (also in `include/util.se`), which builds these arguments from the OS standard streams and calls `__main__`:

* err and out: Sinks that write each value to standard error and standard output (`__Stream_newSink__`, which uses `__Stream_sink__`).
* in: a LazyList of the lines of standard input, each including its newline (`__LazyList_lines__`). A last line without a newline is an element as it is. Standard input is read only as far as the `LazyList` is walked, and only once.
* args: a Seq of the program arguments given on the command line, as Strings.

The test runner does this with `test/vproc_run.py --main <module_file> [<arg> ...]`; without `--main`, it applies a named function to no arguments.

`__main__` calls the user-defined `main` function with the argumentSeq.
The read position of the inputSeq, the outputSink, and the errorSink are passed via `PDict`.
A child started with `"__main__"` may be given `stdin` of its parent as its inputSeq: it reads the same lines, from the parent's read position, and the parent's read position does not move.

Example:

``` scheme
(define main ,(args)
  __print__ "Hello, World!")
```
