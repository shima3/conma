# Design of interpreter the ConMa programming language

## AST

*Distinction between Meta-Location and SourceInfo Data*

The line and column numbers within the **`@line:col`** suffix are always represented as raw numeric values, as they function as structural metadata for debugging.

In contrast, the line and column values within a **`SourceInfo`** node value must be represented as **String literals** (enclosed in double quotes), in accordance with the grammar: `SourceInfo = "(", "__SI__", { String }, ")"`.

Consequently, the output format for a `SourceInfo` node is defined as:
`SourceInfo@line:col: "<filename>" "<line>" "<col>"`

## Execution

If the `__CChain_pop_CFrame__` primitive is invoked when the **Continuation Chain (CChain)** is empty, the following behavior is guaranteed:

1. **Null Passing**: The interpreter passes a `null` value as the argument to the current **Local Continuation (LCont)**.
2. **Execution Resumption**: The `LCont` is then executed with this `null` value, allowing the program to handle the end of the chain or an empty state through standard null-check logic (e.g., using `__is_null__`).

**Operational Note:**
This design ensures that a VP (Virtual Process) does not abruptly terminate or enter an undefined state when the chain is exhausted. Instead, it provides a predictable signal (`null`) to the functional layer, maintaining the language's emphasis on explicit control flow.
