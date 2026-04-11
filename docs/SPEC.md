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
Head       = "(", { Parameter }, ")" ;
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
If the executing VProc becomes blocked (for example, __VProc_suspend__), it is removed from the ready queue until it becomes runnable again.

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

An **argument sequence** and an **input sequence** are of type `Sequence`.
A `Sequence (Seq)` is a function that accepts one Sink.
It applies that Sink to zero or more elements and
returns the final Sink produced by those applications.

A Seq applies its elements to a Sink in order.
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
  lc (,()
    __CChain_set__ cc ,()
    __CChain_pop_LCont__))
```
__Cont_get__ passes the CChain to the LCont.
If you get the continuation at the beginning of the function with __Cont_get__, you can use it as return.

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
  Seq (,(first)
    __CChain_pop_LCont__ ,(rest)
    return first rest) ,(dummy)
  onEmpty)

(define __Seq_get_all__ ,(Seq)
  __CChain_pop_LCont__ ,(lc)
  Seq lc ,(dummy)
  __exit__ "__Seq_get_all__ insufficient number of elements")
```
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
  __Cont_get__ ,(exit)
  __PDict_put__ "__EXIT__" exit ,()
  __PDict_put__ "__STD_IN__" in ,()
  __PDict_put__ "__STD_OUT__" out ,()
  __PDict_put__ "__STD_ERR__" err ,()
  main args ,()
  exit "")
```

* err: a Sink used to send values to the standard error stream.
* args: a Sequence of strings provided to the program at startup.
* in: a Sequence representing the standard input stream.
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
  __PDict_lookup_or_error__ "__STD_IN__")

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

(define __true__ ,(then else)
  __NULL__ then)

(define __false__ ,(then else)
  __NULL__ else)

(define __not__ ,(Boolean)
  Boolean __false__ __true__)
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
The interpreter starts execution by spawning a virtual process that applies `__main__` to the following arguments: an errorSink, an argumentSeq, inputSeq, and an outputSink.

`__main__` calls the user-defined `main` function with the argumentSeq.
The inputSeq, the outputSink, and the errorSink are passed via `PDict`.

Example:

``` scheme
(define main ,(args)
  __print__ "Hello, World!")
```
