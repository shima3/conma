# Specification of the ConMa programming language

The ConMa (abbreviation for Continuation Machine) programming language is based on lambda calculus where all functions are curried in continuation passing style (CPS).
This programming language aims for a concise specification for portability.
It is intended to be used as an intermediate language that translates from more beginner-friendly languages ​​to this language and executes them.
A certain degree of readability is necessary for debugging, but syntactic sugar is not required.
This page describes a specification of the language.

# EBNF Syntax Definition

This document separates the **lexical definitions** (used for tokenization) and **syntactic definitions** (used for parsing).

---

## Lexical Definitions (Tokens)

```ebnf
Token        = Significant | Whitespace | Comment ;
Significant  = Symbol | String | "(" | ")" | "," ;

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

Comment     = LineComment | BlockComment ;
LineComment = ";", { LineCommentCharacter }, Newline ;
LineCommentCharacter = ? any valid character except Newline ? ;

BlockComment = "#|", { BlockCommentContent }, "|#" ;
BlockCommentContent = BlockCommentText | BlockComment ;
BlockCommentText    = ? character sequence not containing #| or |# ? ;
```

## Syntactic Definitions (Grammar)

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
Body       = [ SourceInfo ], Operator, OList, [ LCont ] ;
Operator   = Variable | FuncExp ;
OList      = { Operand } ;
LCont      = Function ;
Operand    = Expression ;
Expression = Variable | String | FuncExp ;
FuncExp    = "(", Function, ")" ;
SourceInfo = "(", "__SI__", { String }, ")" ;
```

---

# Terms

Variable Environment:
An environment is a finite mapping from variable identifiers to values (or closures), used to interpret expressions in a functional programming language.
It captures the lexical (static) bindings of variables at the time of expression evaluation.
It permits bindings to refer to themselves or to each other recursively during evaluation.

Closure:
A closure is a function together with its referencing environment, i.e., the variables that were in scope at the time the function was defined.
It allows the function to access those variables even when it is invoked outside of their original scope.

Call Stack (CS):
A call stack is a special stack data structure that stores information about the active subroutines or function calls of a computer program. It is used to keep track of function execution in the order they are called and to manage the return points when functions complete.

Stack Frame (SF):
A stack frame is a data structure that represents the state of a single function call on the CS. It contains SourceInfo and Closure.
Each time a function is called, a new stack frame is pushed onto the stack; when the function returns, its frame is popped from the stack.

Process Dictionary (PD):
A process dictionary is a key-value storage that is local to a specific process and accessible only within that process’s context.

Virtual Process (VP):
A virtual process is a lightweight, isolated, concurrent unit of execution that runs independently and communicates with other processes via pipes.
Virtual Processes do not share memory with each other.
Each virtual process has its own memory and maintains the following components:

  * SourceInfo
  * Operator
  * OList
  * LCont
  * Variable Environment
  * Call Stack
  * Process Dictionary

Bound Variable (BV):
A Bound Variable is a Variable in the Head of a Function and the same Variable in the Body is BVs within the Function.

Free Variable (FV):
A Free Variable is a Variable that is not BV within an Expression.

For example, the second and subsequent x's and the third and subsequent y's are bound variables, and the rest are free variables within `x y ^(x) x y ^(y) x y`.

Local Continuation (LCont):
The remaining sequence of operations within the current activation record (stack frame), excluding any control flow that returns to the caller.

Reference cell (Ref):
A reference cell is a mutable container that holds a single value, providing a stable identity to a piece of data that can change over time.
It decouples the identity of the container from its content, allowing programs to manage side-effects and shared state through explicit dereferencing and atomic updates.

# Operational Semantics

SourceInfo (SI) is information for debugging purposes and does not affect basic operation. It is used to indicate the location in the source code when an error occurs or some debuggers are running the program.

The Includer (include FileName) includes the Program from the file named FileName at loding time.
ConMa source files are expected to have the `.se` extension.

* `include "filename"`:
    The interpreter recursively scans and loads the specified file.
    If "filename" is provided without an extension, the interpreter searches for "filename.sch".

The Definition (define Variable Function) binds the identifier Variable to the closure of Function, thereby defining a new named function in the Environment.

# EBNF Notation Used

The EBNF (Extended Backus-Naur Form) notation used in the definitions adheres to common conventions for describing the syntax of formal languages.
Below is an explanation of the elements and constructs in this EBNF:

## Key Elements and Constructs

### Rule Definition

Syntax rules are defined with the format:

```ebnf
RuleName = Expression ;
```

- The left-hand side (`RuleName`) specifies the name of the rule.
- The right-hand side (`Expression`) defines its structure.

### Concatenation

Elements listed sequentially must appear in the given order:

```ebnf
Expression = Element1, Element2 ;
```

- Example: `Element1` is followed by `Element2`.

### Alternatives

Multiple options are separated by `|`, indicating that any one of them can match:

```ebnf
Expression = Option1 | Option2 ;
```

- Example: Matches either `Option1` or `Option2`.

### Repetition

`{ ... }` indicates zero or more repetitions of the enclosed expression:

```ebnf
Expression = { Element } ;
```

- Example: Matches `Element` repeated zero or more times.

### Optional Elements

`[ ... ]` indicates that the enclosed expression is optional (zero or one occurrence):

```ebnf
Expression = [ Element ] ;
```

- Example: Matches `Element` if present, but it can be omitted.

### Groupings

Parentheses `( ... )` are used to group expressions and clarify precedence:

```ebnf
Expression = ( Element1, Element2 ) ;
```

- Example: Matches `Element1` followed by `Element2` as a single grouped unit.

### Terminal Symbols

Strings or characters enclosed in quotation marks represent literal values:

```ebnf
Terminal = "literal" ;
```

- Example: Matches the exact string `"literal"`.

### Character Ranges

Ranges specify a set of characters using the `...` operator:

```ebnf
Digit = "0"..."9" ;
```

- Example: Matches any single digit between `0` and `9`.

### Special Characters

Some non-printable or special characters (e.g., whitespace, newlines) are represented with descriptive names:

```ebnf
Newline = "\n" | "\r\n" ;
```

- Example: Matches a line feed (`\n`) or a carriage return followed by a line feed (`\r\n`).

### Exception (Subtraction)

Exception indicates that the elements following the minus sign are excluded from the set of elements defined before it.

```ebnf
Rule = SetA - SetB ;
```

---

# Syntax

In `Body`, the first `Expression` is the **Operator**, and the second and subsequent `Expression`s form the **OList**.

**LCont Rules:**
- Any `Function` intended to be an `Operand` must be wrapped in parentheses as a `FuncExp`.
- An unparenthesized `^` symbol signals the end of the `OList` and the start of the `LCont`.

In the first version of the language:

* Top-level expression evaluation is not allowed. Only `Statement`s are permitted at the top level.
* Numeric literals are not supported.

---

## Evaluation Strategy

The Hat interpreter employs a strict **Call-by-Value** strategy for `Variable` and `String` types, and a **Call-by-Name (Thunk-based)** strategy for `FuncExp`.

### 1. Argument Processing (Eager vs. Lazy)

Before a function application occurs, each `Operand` in the `OList` is processed into a **Value** based on its syntactic category:

* **Variable**: Evaluated immediately. The identifier is replaced by the value currently bound to it in the `Variable Environment`.
* *Note on `Ref`:* If a variable is bound to a `Ref` (reference cell), the `Ref` itself is passed as the value. The content inside the `Ref` is **not** dereferenced during this stage.

* **String**: Treated as a literal value and passed as-is.
* **FuncExp**: Not executed. It is captured along with the current `Variable Environment` to create a **Closure** (Thunk). This closure is passed as the value, and its internal `Body` is only evaluated if/when explicitly invoked by the callee.

### 2. Evaluation Order and Side Effects

Because `Variable` and `FuncExp` are resolved into values (either raw data or closures) before the function body is entered, the execution of the function is **order-independent** regarding its arguments. No side effects occur during the argument resolution phase itself, as dereferencing or state mutation only happens via explicit internal primitives (e.g., `__Ref_get__`, `__Ref_set__`).

### 3. Application Process

Once the `Operator` and processed `OList` are ready:

1. **Operator Resolution**: The `Operator` is resolved to a function or closure.
2. **Binding**: The processed values from `OList` are bound to the `Parameter` identifiers in the function's `Head`.
3. **LCont Handling**: If a `LCont` (Local Continuation) is present, a `Stack Frame` containing the `LCont`'s closure and current `SourceInfo` is pushed onto the `Call Stack`.
4. **Body Execution**: The `Body` of the function is evaluated within the new environment.

Example:

```
(^(p1 p2 p3 ...) B) A1 "A2" (^() A3)
```

Evaluation proceeds as follows:

* The value bound to `A1` is assigned to `p1`.
* The string `"A2"` is assigned to `p2`.
* The closure of `(^() A3)` is assigned to `p3`.

After these assignments, `B` is evaluated.

---

# Scope

Recursive definitions are permitted using `define`.
Example:
```
(define f ^(x) x (f x))
```

Shadowing is permitted.
Example:
```
^(x) (^(x) x)
```
is semantically equivalent to:
```
^(x) (^(x2) x2)
```
---

# Execution Model and CPS Semantics

A process managed by the operating system is called an **OS Process**.
A process managed by the interpreter is called a **Virtual Process (VP)**.

In the first version:
A **Virtual Process (VP)** is scheduled using round-robin scheduling.
The scheduling granularity is **one Operator application**.

A single scheduling step is defined as follows.

1. The scheduler selects the next runnable VP from the ready queue.
2. The selected VP executes exactly **one Operator application**.
3. After the Operator application completes, control returns to the scheduler.
4. If the VP is still runnable, it is placed at the end of the ready queue.
5. The scheduler then selects the next VP in the queue.

An **Operator application** is defined as the evaluation of one `Body`:
```
[ SourceInfo ], Operator, OList, [ LCont ]
```

The execution of an Operator application consists of:

1. Determining the function designated by `Operator`.
2. Assigning the values of `OList` to the corresponding parameters.
3. Creating and pushing a Stack Frame if required by the CPS rules.
4. Beginning execution of the function body.

An Operator application is considered complete immediately before the next Operator to be evaluated becomes the current Operator.
Invoking LCont is considered part of the following Operator application.
Internal primitives are executed as one Operator application.

At that point, the scheduler may switch execution to another VP.
If the executing VP becomes blocked (for example, __VP_suspend__), it is removed from the ready queue until it becomes runnable again.

The function designated as the `Operator` in `Body` is applied to `OList`.
A Stack Frame consisting of `SourceInfo` and the closure of `LCont` is pushed onto the Call Stack.

In normal mode:
* If there is no LCont, no Stack Frame is pushed.
* Therefore, tail recursion does not consume Call Stack space.

In debug mode:
* Even if there is no LCont, a Stack Frame consisting of `SourceInfo` and a closure of __noop__ is pushed.
* __noop__ performs no operation.
* This Stack Frame exists solely to preserve `SourceInfo` for stack trace generation.

---

## Sinks and Sequences

An **output sink** and an **error sink** are of type `Sink`.
A `Sink` is a function that accepts an argument and returns a `Sink`.

An **argument sequence** and an **input sequence** are of type `Sequence`.
A `Sequence (Seq)` is a function that accepts one Sink.
It applies that Sink to zero or more elements and
returns the final Sink produced by those applications.

A Seq applies its elements to a Sink in order.
The grouping of arguments in each call is implementation-defined, provided that the observable result is equivalent to successive single-argument applications.

Example:
(define sample_sink ^(Value)
  print Value ^()
  sample_sink)

(define sample_seq ^(sink)
  sink "A" "B" "C" ^(last_sink)
  last_sink)

---

# Internal Primitives

### `__OS_spawn__ ErrorSink CommandName ArgumentSequence InputSequence OutputSink`
Behavior:
Creates and executes a new OS Process.

---

### `__VP_spawn__ ErrorSink SourceFileName ArgumentSequence InputSequence OutputSink`
Behavior:
Initiates the creation and execution of a new Virtual Process (VP) through the following sequential phases:

1. **Process Initialization**: A new, isolated Virtual Process is allocated with its own independent memory, Call Stack, and Process Dictionary.
2. **Static Analysis and Loading**: The interpreter loads the program from the file specified by `SourceFileName`. During this loading phase, the interpreter performs a **global scan** of all `define` statements within the file and any included files.
3. **Environment Binding**: All identifiers identified during the scan are registered in the new VP’s **Variable Environment**. This ensures that named functions can refer to each other recursively or out of order from the moment execution begins.
4. **Entry Point Execution**: The interpreter invokes the `main` function of the loaded program by applying the `__MAIN__` wrapper.
5. **Context Injection**: The `__MAIN__` function is applied to the following arguments:
* **ErrorSink**: For handling standard error output.
* **ArgumentSequence**: For program startup arguments.
* **InputSequence**: Representing the standard input stream.
* **OutputSink**: For standard output.

---

### `__VP_current__`
Behavior:
Passes the current VP to the LCont.

---

### `__VP_suspend__ VP`
Behavior:
Suspends the `VP`.
Moves the `VP` from the ready queue to the blocked queue.

---

### `__VP_resume__ VP`
Behavior:
Resumes the `VP`.
Moves the `VP` from the blocked queue to the ready queue.

---

### `__SF_new__ FuncExp SourceInfo`
Behavior:
Creates a Closure from `FuncExp` and the current variable environment, then creates a Stack Frame containing that Closure and `SourceInfo`, and passes the Stack Frame to the LCont.

---

### `__SF_get_Closure__ StackFrame`
Behavior:
Passes the Closure stored in `Stack Frame` to the LCont.

---

### `__CS_get__`
Behavior:
Passes the current CS to the LCont.

---

### `__CS_set__ CS`
Behavior:
Sets the CS as the current CS.

---

### `__CS_push_SF__ StackFrame`
Behavior:
Pushes `Stack Frame` onto the Call Stack.

---

### `__CS_pop_SF__`
Behavior:
Pops a Stack Frame from the Call Stack and passes it to the LCont.

---

### `__PD_put__ Key Value`
Behavior:
Associates the `Value` with the `Key` in the Process Dictionary.

---

### `__PD_get__ Key`
Behavior:
Retrieves the value for the `Key` in the Process Dictionary.
If there is not value, it returns null.

---

### `__Ref_new__ Value`
Behavior:
Creates a Ref, sets the `Value`, and passes it to the LCont.

### `__Ref_get__ Ref`
Behavior:
Gets the `Value` for the `Ref` and passes it to the LCont.

### `__Ref_set__ Ref Value`
Behavior:
Sets the `Value` for the `Ref`.

---

### `__CS_pop_LCont__`
Behavior:
pops a parent LCont and passes it to the current LCont.
It is defined as follow:
```
(define __CS_pop_LCont__ ^()
  __CS_pop_SF__ ^(sf)
  __CS_pop_SF__ ^(sf2)
  __SF_get_Closure__ sf ^(lc)
  __SF_get_Closure__ sf2 ^(lc2)
  lc lc2)
```
---

### `noop`
Behavior:
Do nothing.
It is defined as follow:
```
(define __noop__ ^()
  __CS_pop_LCont__ ^(lc)
  lc)
```
---

### `__is_null__ Value`
Behavior:
Passes `(^(t f) t)` to the LCont if the `Value` is null, otherwise passes `(^(t f) f)`.

---

### `__NULL__`
A variable defined as null.

---

# Standard Definition

### `Continuation`
```
(define __Cont_get__ ^()
  __CS_pop_LCont__ ^(lc)
  __CS_get__ ^(cs)
  lc (^()
    __CS_set__ cs ^()
    __CS_pop_LCont__))
```
__Cont_get__ passes the Continuation to the LCont.
If you get the continuation at the beginning of the function with __Cont_get__, you can use it as return.

Example:

(define f ^()
  __Cont_get__ ^(return)
  g ^(x y)
  stdout x ", " y ^(out)
  return)

(define g ^()
  __Cont_get__ ^(return)
  return "12" "34")

---

### `Sequence`
```
(define __Seq_push__ ^(First Rest Sink)
  Sink First ^(sink)
  Rest sink)

(define __Seq_get__ ^(Seq)
  __Cont_get__ ^(return)
  Seq (^(first)
    __CS_pop_LCont__ ^(rest)
    return first rest) ^(dummy)
  return __NULL__)

(define __Seq_get_all__ ^(Seq)
  __CS_pop_LCont__ ^(lc)
  Seq lc ^(dummy)
  __exit__ "__Seq_get_all__ insufficient number of elements")
```
---

### `Stream`
```
(define __Seq_stream__ ^(Seq)
  __Ref_new__ Seq ^(ref)
  (^(sink)
    __Ref_get__ ref ^(seq)
    __Seq_get__ seq ^(first rest)
    __Ref_set__ ref rest ^()
    sink first ^(sink2)
    __Ref_get__ ref ^(rest2)
    rest2 sink2))

(define __Sink_stream__ ^(Sink)
  __Ref_new__ Sink ^(ref)
  (^(value)
    __Ref_get__ ref ^(sink)
    sink value ^(sink2)
    __Ref_set__ ref sink2 ^()
    sink2))
```
---

### `Virtual Process`
```
(define __VP_wait_until__ ^(cond)
  __VP_current__ ^(proc)
  __fix__ (^(loop)
    cond ^()
    __VP_suspend__ proc ^()
    loop))
```
---

### `Ref`
```
(define __Ref_wait_until__ ^(ref cond)
  __VP_wait_until__ (^()
  __Ref_get__ ref ^(value)
  cond value))

(define __Ref_wait_for_null__ ^(ref)
  __Cont_get__ ^(return)
  __Ref_wait_until__ ref (^(value)
    if (^() __is_null__ value) return))

(define __Ref_wait_for_non_null__ ^(ref)
  __Cont_get__ ^(return)
  __Ref_wait_until__ ref (^(value)
  unless (^() __is_null__ value) (^() return value)))
```

### `Pipe`
```
(define __Pipe_new__ ^()
  __Ref_new__ __NULL__ ^(inProcRef)
  __Ref_new__ __NULL__ ^(outProcRef)
  __Ref_new__ __NULL__ ^(valueRef)
  (^(sink) sink inProcRef outProcRef valueRef))

(define __Pipe_in__ ^(Pipe Sink)
  __VP_current__ ^(inProc)
  __Seq_get_all__ Pipe ^(inProcRef outProcRef valueRef)
  __Ref_set__ inProcRef inProc ^()
  __Ref_wait_for_non_null__ outProcRef ^(outProc)
  __Ref_get__ valueRef ^(value)
  __Ref_set__ inProcRef __NULL__ ^()
  __VP_resume__ outProc ^()
  __Ref_wait_for_null__ outProcRef ^()
  Sink value)

(define __Pipe_out__ ^(Pipe Value)
  __VP_current__ ^(outProc)
  __Seq_get_all__ Pipe ^(inProcRef outProcRef valueRef)
  __Ref_set__ valueRef Value ^()
  __Ref_set__ outProcRef outProc ^()
  __Ref_wait_for_non_null__ inProcRef ^(inProc)
  __VP_resume__ inProc ^()
  __Ref_wait_for_null__ inProcRef ^()
  __Ref_set__ outProcRef __NULL__)

(define __Pipe_new_in_out__ ^()
  __Cont_get__ ^(return)
  __Pipe_new__ ^(pipe)
  __Pipe_in__ pipe ^(in)
  __Pipe_out__ pipe ^(out)
  return in out)
```
---

### `MAIN`
```
(define __MAIN__ ^(err args in out)
  __Cont_get__ ^(exit)
  __PD_put__ "__STD_IN__" in ^()
  __PD_put__ "__STD_OUT__" out ^()
  __PD_put__ "__STD_ERR__" err ^()
  __PD_put__ "__EXIT__" exit ^()
  main args ^()
  exit __NULL__)
```
* err: a Sink used to send values to the standard error stream.
* args: a Sequence of strings provided to the program at startup.
* in: a Sequence representing the standard input stream.
* out: a Sink used to send values to the standard output stream.

---

### `Standard input, output, error`
```
(define stdin ^()
  __PD_get__ "__STD_IN__")

(define stdout ^()
  __PD_get__ "__STD_OUT__")

(define stderr ^()
  __PD_get__ "__STD_ERR__")
```
---

### `exit`
```
(define __exit__ ^(String)
  __PD_get__ "__EXIT__" ^(exit)
  exit String)
```

### `print`
```
(define __print__ ^(str)
  __CS_pop_LCont__ ^(lc)
  stdout str "\n" ^(out)
  lc)
```
---

### `Control flow`
```
(define __if_then_else__ ^(flag then else)
  flag then else ^(clause)
  clause)

(define __if__ ^(flag then)
  __if__then_else__ flag then __noop__)

(define __unless__ ^(flag else)
  __if__then_else__ flag __noop__ else)

(define __fix__ ^(x) x (__fix__ x))

(define __true__ ^(then else)
  then)

(define __false__ ^(then else)
  else)

(define __not__ ^(flag)
  flag __false__ __true__)
```

# Error Handling

* If an unbound variable is encountered, an error is displayed and execution terminates.
* If too few arguments are supplied, partial application occurs and this is not treated as an error.
* If too many arguments are supplied, a local continuation that applies the remaining arguments is pushed onto the Call Stack, and this is not treated as an error.

Example: (^(p1) p1) a1 a2 -> (^(f) f a2) a1
(^(f) f a2) is the local continuation.

* If the Operator is a Variable, it replaces with the Value assigned it in Variable Environment.
* If the Operator is a FuncExp, it replaces with the closure of the FuncExp.
* If the Operator is a closure, it applies the closure to the OList.
* If the Operator is not a Variable, a FuncExp, or a closure, an error is displayed and execution terminates.

---

# Program Entry Point: `main`

A Hat program must define a `main` function.
The interpreter starts execution by spawning a virtual process that applies `__MAIN__` to the following arguments: an ErrorStream, an ArgumentSeq, an InputStream, and an OutputStream.

`__MAIN__` calls the user-defined `main` function with the ArgumentSeq.
the InputStream, the OutputStream, and the ErrorStream are passed via PD.

Example:
(define main ^(args)
  __print__ "Hello, World!")

# Reserved Variables

Any Symbol other than `^` is a variable.
Symbols with two or more characters, including a character `^`, are variables and are reserved.
