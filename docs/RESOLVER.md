## Specification of `resolver`

### Overview

`resolver` reads the AST stream produced by `parser`, resolves all `Variable` references to either local or global bindings, annotates each `Variable` node with its binding kind and index, and writes the transformed AST to standard output followed by a global variable table. Errors are written to standard error.

This component is the fourth stage of the processing pipeline:

```
lexer  →  comment_remover  →  parser  →  resolver
```

---

### Invocation

```
resolver
```

The program reads from standard input. There are no command-line options. The output is written to standard output.

---

### Input Format

The input must be the AST stream produced by `parser`, in the format defined by `AST.md` and `PARSER.md`. Each line is one of:

- A **node line**: `[whitespace] NodeType@line:col[: value]`
- An **End tag line**: `[whitespace] End@line:col: NodeType`
- An empty line (ignored)

The node type name consists of ASCII letters, digits, and underscores, starting with a letter or underscore. The `line` and `col` fields are non-negative integers. The `value` field, if present, is the text after `": "` to the end of the line.

The program may receive multiple `Program` nodes in sequence (multi-file output from `includer`).

---

### Output Format

The output is the transformed AST stream followed by the global variable table, both written to standard output.

#### Transformed AST

The transformed AST has the same structure and format as the input, with one change: every `Variable` node's annotation is replaced with a binding annotation of the following form:

```
Variable@line:col: <name> (<tag><index>)
```

- `<name>`: the variable name as it appears in the source, without enclosing quotes.
- `<tag>`: `L` if the variable is locally bound, `G` if it is globally bound.
- `<index>`: a non-negative integer. For local variables, this is the de Bruijn index (see **Local Variable Resolution**). For global variables, this is the global sequence number (see **Global Variable Resolution**).

All other node types are written unchanged, including their End tags.

#### Global Variable Table

After all transformed AST output, the following header line is written:

```
Global Table
```

Then, for each global variable in the order it was first assigned a sequence number, one tab-separated line is written:

```
<id> TAB <name> TAB <value_type>
```

- `<id>`: the global sequence number of the variable (0-based integer).
- `<name>`: the variable name.
- `<value_type>`: the node type of the value node of the `Definition` that defines this variable (e.g., `Function`). If the variable appears in the AST but is not defined by any `Definition` node, this field is `Undefined`.

---

### Processing

Processing proceeds in two passes over all `Program` trees, followed by output.

#### AST Reconstruction

Before the two passes, the input stream is parsed into a forest of `ASTNode` trees. The reconstruction algorithm is as follows:

- A node stack is maintained, initially empty.
- Each node line creates an `ASTNode` with the given type, line, col, and value.
  - If the stack is empty, the new node is added to the root list.
  - Otherwise, the new node is appended to the children of the node at the top of the stack.
  - If the node type is not a terminal type, the new node is pushed onto the stack.
- Each End tag line pops one node from the stack (if the stack is non-empty). The End tag's type and coordinates are not validated against the popped node.
- Empty lines and lines that do not match the node pattern are ignored.

The terminal node types — those that are never pushed onto the stack and therefore never have children — are:

`Variable`, `String`, `Null`, `SourceInfo`, `FileName`

#### Pass 1: Definition Collection

Pass 1 traverses all `Program` trees recursively (depth-first, pre-order). For each `Definition` node encountered:

1. The first child of `Definition` whose type is `Variable` is taken as the defined name. Its value is unquoted (enclosing `"` characters are stripped) to obtain the name string.
2. The second child of `Definition` (index 1), if present, is recorded as the value node for that name.
3. If the name has not been seen before, it is registered in the global registry with `id = None` and the value node from step 2. If the name has already been registered, it is not re-registered.

Pass 1 does not assign global sequence numbers.

#### Pass 2: Scope Resolution

Pass 2 traverses each `Program` tree recursively (depth-first, pre-order) with a **scope stack**, initially empty. The scope stack is a list of scopes; each scope is an ordered list of parameter name strings. The bottom of the stack corresponds to the outermost enclosing function.

**On entering a `Function` or `Lambda` node:**
The `Head` child (if present) is examined. The value of each `Variable` child of `Head`, unquoted, is collected in order into a new scope list. This scope list is pushed onto the scope stack before the children of `Function`/`Lambda` are visited.

**On leaving a `Function` or `Lambda` node:**
The scope list that was pushed on entry is popped from the scope stack.

**On visiting a `Variable` node:**
The variable name is obtained by unquoting the node's value. The name is searched in the scope stack as follows:

*Local search*: the scope stack is examined from top (innermost) to bottom (outermost). For each scope (examined in innermost-first order), an `accumulated_binders` counter starts at 0 before the search begins and increases by the length of each scope that does not contain the name.

When the name is found in a scope:
- The position within that scope is determined as the 0-based index from the **right end** of the scope list. That is, if the scope is `[p, q, r]`, then `r` has position 0, `q` has position 1, and `p` has position 2.
- The de Bruijn index is `accumulated_binders + position`.
- The variable is marked local with this index.

*Global resolution*: if the name is not found in any scope:
- If the name is not in the global registry, it is added with `id = None` and `value = None`.
- If the name's global sequence number has not yet been assigned (`id` is `None`), it is assigned the current value of the global counter (starting at 0) and the counter is incremented by 1.
- The variable is marked global with its assigned sequence number.

Children of a `Variable` node are not visited (terminal node).

---

### De Bruijn Index

The de Bruijn index used here is the standard nameless representation for lambda calculus:

- Index 0 refers to the parameter of the immediately enclosing function.
- Index 1 refers to the parameter of the next enclosing function, and so on.

Since all ConMa functions are curried (each `Head` binds exactly one parameter in practice), each scope on the stack contains exactly one name and the index equals the nesting depth of the binding function counted from the innermost enclosing one.

When a `Head` contains multiple parameters (non-standard usage), parameters are indexed right-to-left within the same scope before moving to outer scopes.

---

### Example

#### Source (`sample.se`)

```scheme
(define main ,(args)
  __print__ "Hello")
```

#### Input (from `parser --file sample.se`)

```
Program@1:1
  FileName@0:0: "sample.se"
  Definition@1:2
    Variable@1:9: main
    Function@1:14
      Head@1:15
        Variable@1:16: args
      End@1:15: Head
      Body@2:3
        SourceInfo@2:3: "sample.se" "2" "3"
        Operator@2:3
          Variable@2:3: __print__
        End@2:3: Operator
        OList@2:13
          String@2:13: "Hello"
        End@2:13: OList
        LCont@2:20
          Null@2:20
        End@2:20: LCont
      End@2:3: Body
    End@1:14: Function
  End@1:2: Definition
End@1:1: Program
```

#### Output

```
Program@1:1
  FileName@0:0: "sample.se"
  Definition@1:2
    Variable@1:9: main (G0)
    Function@1:14
      Head@1:15
        Variable@1:16: args (L0)
      End@1:15: Head
      Body@2:3
        SourceInfo@2:3: "sample.se" "2" "3"
        Operator@2:3
          Variable@2:3: __print__ (G1)
        End@2:3: Operator
        OList@2:13
          String@2:13: "Hello"
        End@2:13: OList
        LCont@2:20
          Null@2:20
        End@2:20: LCont
      End@2:3: Body
    End@1:14: Function
  End@1:2: Definition
End@1:1: Program
Global Table
0	main	Function
1	__print__	Undefined
```

---

### Notes

- The `Variable` node that names a `Definition` (the defined symbol itself) is resolved by the same rules as any other `Variable`. In Pass 2 it is visited before `Function` body is entered, so it is resolved as global (no enclosing function scope is active at the `Definition` level).
- A variable name may appear in the global registry with `id = None` after Pass 1 if it was registered during definition collection. Its sequence number is assigned lazily in Pass 2 when the name is first encountered as a free variable.
- The global sequence numbers are assigned in the order variables are first encountered as free (global) references during Pass 2's depth-first traversal, not in the order of `Definition` nodes.
