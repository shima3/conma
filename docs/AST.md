## AST Output Specification

### 1. General Principles

- **Location information is always attached**: every node name is followed by `@line:col`, where `line` and `col` are the 1-based line and column numbers of the first character of the token that introduced the node.
- **Automatic SourceInfo insertion**: when the `--file <filename>` option is given, the parser inserts a `SourceInfo` node into every `Body` that does not already contain an explicit `(__SI__ ...)` construct.

---

### 2. Node Format

```
NodeType@line:col [: value]
```

- `NodeType`: one of the node kinds defined in Section 3.
- `line`: 1-based line number of the first character of the corresponding token.
- `col`: 1-based column number of the first character of the corresponding token.
- `: value`: present only for terminal nodes (`Variable`, `String`, `SourceInfo`, `FileName`). For `SourceInfo`, the value is a space-separated sequence of string literals. For `Variable`, `String`, and `FileName`, the value is the exact text enclosed in double quotes.

Child nodes are indented by two spaces relative to their parent.

**Note on Data Types:**
* The coordinates in the `@line:col` suffix are **numeric**.
* The values within a `SourceInfo` node are **Strings**, meaning the line and column must be enclosed in double quotes (e.g., `"10"`), matching the EBNF definition.

---

### 3. Node Kinds

| NodeType | Corresponding grammar element | Terminal? |
|---|---|---|
| `Program` | Program | No |
| `FileName` | — (meta node) | Yes (value = quoted source filename) |
| `Definition` | Definition | No |
| `Includer` | Includer | No |
| `Function` | Function | No |
| `Head` | Head | No |
| `Body` | Body | No |
| `SourceInfo` | SourceInfo | Yes (value = string literals) |
| `Operator` | Operator | No |
| `OList` | OList | No |
| `LCont` | LCont | No |
| `Variable` | Variable | Yes (value = symbol text) |
| `String` | String | Yes (value = quoted string text) |
| `FuncExp` | FuncExp | No |
| `Null` | absent optional element | Yes (no value) |

---

### 4. Structural Rules

1. `Operator`, `OList`, and `LCont` are always emitted as explicit nodes, even when `OList` contains no elements.
2. `Head` is always emitted, even when it contains no parameters.
3. When `LCont` is absent in the source, a `Null` node is emitted in its place, carrying the location of the character immediately following the last token of `OList`.
4. When `--file <filename>` is specified, a `SourceInfo` node of the following form is inserted as the first child of every `Body` that does not already contain an explicit `(__SI__ ...)` construct:
   ```
   SourceInfo@line:col: "<filename>" "<line>" "<col>"
   ```
   where `line` and `col` refer to the location of the `Operator` within that `Body`.
   *(Note: Ensure line/col are quoted as strings here)*
5. When `(__SI__ ...)` is already present in the source, the `--file` option does not override it. The existing `SourceInfo` node takes precedence.
6. For abstract structural nodes that have no direct corresponding token (`OList`, `LCont`, `Body`, etc.), the location is that of the first token of the node's content. If the node has no content (e.g., an empty `OList`), the location is that of the token immediately following the node's syntactic position.

---

### 5. Multi-File Output

When multiple source files are processed (e.g., by a driver that resolves `Includer` nodes), each source file produces one `Program` node. The `Program` nodes are written to standard output in the order the source files were processed, with no separator or enclosing node between them.

Each `Program` node is formatted at indentation level 0, exactly as it would be in single-file output. The output of one `Program` ends immediately before the first character of the next.

#### Example: two files processed in order

```
Program@1:1
  FileName@0:0: "a.se"
  Definition@1:2
    ...
Program@1:1
  FileName@0:0: "b.se"
  Definition@1:2
    ...
```

A consumer of this output must treat each top-level line that matches `Program@...` as the start of a new `Program` node. The `FileName` node, when present, is always the first child of `Program`.

---

### 6. Example

#### Source (`test.se`)

```scheme
(include "util.se")
(define main ,(args)
  __print__ "Hello")
```

#### Output: `parser --file test.se`

```
Program@1:1
  FileName@0:0: "test.se"
  Includer@1:1
    String@1:10: "util.se"
  Definition@2:2
    Variable@2:9: main
    Function@2:14
      Head@2:15
        Variable@2:16: args
      Body@3:3
        SourceInfo@3:3: "test.se" "3" "3"
        Operator@3:3
          Variable@3:3: __print__
        OList@3:13
          String@3:13: "Hello"
        LCont@3:20
          Null@3:20
```

