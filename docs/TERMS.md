# Terms

Variable Environment (VEnv):
An environment is a finite mapping from variable identifiers to values (or closures), used to interpret expressions in a functional programming language.
It captures the lexical (static) bindings of variables at the time of expression evaluation.
It permits bindings to refer to themselves or to each other recursively during evaluation.

Closure:
A closure is a function together with its referencing environment, i.e., the variables that were in scope at the time the function was defined.
It allows the function to access those variables even when it is invoked outside of their original scope.

Process Dictionary (PDict):
A process dictionary is a key-value storage that is local to a specific process and accessible only within that process’s context.

Virtual Process (VProc):
A virtual process is a lightweight, isolated, concurrent unit of execution that runs independently and communicates with other processes via pipes.
Virtual Processes do not share memory with each other.
Each virtual process has its own memory and maintains the following components:

  * SInfo
  * Operator
  * OList
  * LCont
  * VEnv
  * CChain
  * PDict

Bound Variable (BV):
A Bound Variable is a Variable in the Head of a Function and the same Variable in the Body is BVs within the Function.

Free Variable (FV):
A Free Variable is a Variable that is not BV within an Expression.

For example, the second and subsequent x's and the third and subsequent y's are bound variables, and the rest are free variables within `x y ,(x) x y ,(y) x y`.

Reference cell (Ref):
A reference cell is a mutable container that holds a single value, providing a stable identity to a piece of data that can change over time.
It decouples the identity of the container from its content, allowing programs to manage side-effects and shared state through explicit dereferencing and atomic updates.

Local Continuation (LCont):
A **local continuation** is the remaining sequence of operations within the *current activation record*, expressed as a syntactic component of a `Body`, and automatically pushed onto the Continuation Chain as a closure when the operator application is dispatched.

- **Encoded statically** in the syntax of `Body` as an optional trailing `Function`.
- **Not explicitly captured**: it is not obtained via a control operator; it is a structural part of every operator application.
- **Scope is activation-record-local**: its extent is strictly bounded to the current stack frame and does not reach into or past the caller's frame.
- **Consumed once** (as a syntactic construct): each `LCont` is pushed onto the Continuation Chain exactly once, as a structural consequence of the operator application it belongs to. This says nothing about how many times the resulting CFrame — once on the chain — may be invoked if the chain is captured and replayed.

Note: the "single-use" property of `LCont` describes how a local continuation enters the chain (exactly once, automatically). It does not constrain the chain itself: a `CChain` captured via `__CChain_get__` is a first-class value that may be stored and invoked multiple times, enabling non-local jumps and resumable continuations.

Continuation Chain (CChain):
A Continuation Chain is an ordered sequence of CFrames representing the pending computations of the current Virtual Process.
In a pure CPS (Continuation Passing Style) system, there is no "return" in the traditional sense; there is only "calling the next continuation."

* **Structure:** A linked list of **CFrame**. Each closure contains the function code and its referencing environment.
* **Storage:** Continuations are typically allocated on the **Heap**, not a stack. This allows them to persist even after the function that created them has finished its immediate task.
* **Lifecycle:** A continuation exists as long as there is a reference to it. This allows for advanced control flow like non-local jumps, backtracking, or pausing/resuming Virtual Processes.
* **Flexibility:** Because continuations are first-class objects, you can save a "chain," duplicate it, or invoke it multiple times.

Continuation Frame (CFrame):
A CFrame is a data structure that represents the state of a single function call on the CChain.
It contains Closure, SInfo, and the next CFrame.
Each time a function is called, a new CFrame is pushed onto the CChain; when the function returns, its frame is popped from the CChain.

null:
A unique value representing the absence of a value.

**Mutable Sequence (MSeq)**:
A MSeq is a finite, ordered collection of zero or more elements that supports in-place modification and dynamic growth. It is defined by the following formal characteristics:

1. **Finiteness and Emptiness Predicate**:
The sequence contains a discrete, countable number of elements ($n \ge 0$). It must provide a predicate (e.g., `is_empty`) to determine if $n=0$. An MSeq with $n=0$ is an **empty sequence**.

2. **Head Access**:
The sequence provides an operation to retrieve the first element (the **head**). If the sequence is not empty, it returns the head element. Otherwise, the behavior is undefined or results in an error.

3. **Tail Access**:
The sequence provides an operation to retrieve all elements following the head as a new MSeq (the **tail**). If the sequence is not empty, it returns the tail. Otherwise, the behavior is undefined or results in an error.

4. **Conditional Head Mutability (set-head)**:
The sequence supports a "set-head" operation with the following semantics:
* **Update**: If the sequence is not empty, the current head is replaced with a new value via a destructive update. Otherwise, the behavior is undefined or results in an error.

5. **Growth (Appendability)**:
The sequence supports the addition of new elements to its end (the **append** operation). This operation increases the total count of elements ($n$) in the MSeq.

6. **Implementation Agnosticism**:
This term defines a behavioral contract and functional interface. It does not mandate a specific memory layout and can be implemented using structures such as dynamic arrays, circular buffers, or linked lists.
---

# Appendix

A partial continuation is a **runtime-captured, prompt-delimited, reusable slice** of the continuation chain, obtained by an explicit control operation.
A local continuation is a **statically-encoded, activation-record-bounded, single-use** trailing computation, which is an implicit structural consequence of every operator application — not the result of any capture operation.
The two concepts are orthogonal: a local continuation describes *where in the syntax* the remaining work resides within one frame; a partial continuation describes *which segment of the runtime continuation chain* has been captured across potentially many frames.
