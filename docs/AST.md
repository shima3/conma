## AST Output Specification

### 1. General Principles

* **Location information is always attached**: every node carries a coordinate tuple `(line col)` as its first child element, where `line` and `col` are the 1-based line and column numbers of the first character of the token that introduced the node.
* **Automatic SInfo insertion**: when the `--file <filename>` option is given, the parser inserts a `SInfo` node into every `Body` that does not already contain an explicit `(__SInfo__ ...)` construct.
* **Node Boundaries**: Non-terminal nodes are explicitly closed with a matching `End` tag to clearly define the tree structure.

---

### 2. Node Format

All nodes are written as S-expressions. Child nodes are indented by 2 spaces. Consecutive closing parentheses on separate lines are merged onto the preceding line.

#### Terminal Nodes

| Node | Format |
|---|---|
| `Variable` | `(Variable (line col) "name")` |
| `String` | `(String (line col) "value")` |
| `SInfo` | `(SInfo (line col) "file" "line" "col")` |
| `Null` | `(Null (line col))` |

#### Program Node

The filename, when present, is embedded as a third element in the coordinate tuple:

```scheme
(Program (line col "filename")
  children...)
```

If no filename is available, the coordinate tuple contains only two elements: `(line col)`.

#### Non-terminal Nodes

```scheme
(NodeType (line col)
  children...)
```

Non-terminal nodes always emit a closing `)`, even when they have no children.

---

### 3. Node Kinds

| NodeType | Corresponding grammar element | Terminal? |
| --- | --- | --- |
| `Program` | Program | No |
| `FileName` | — (meta information) | Yes (value = filename) |
| `Definition` | Definition | No |
| `Includer` | Includer | No |
| `Function` | Function | No |
| `Head` | Head | No |
| `Body` | Body | No |
| `SInfo` | SInfo | Yes (value = string literals) |
| `Operator` | Operator | No |
| `OList` | OList | No |
| `LCont` | LCont | No |
| `Variable` | SYMBOL token | Yes (value = name) |
| `String` | STRING token | Yes (value = text) |
| `Null` | — (empty element) | Yes |

---

### 4. Structural Rules

1. **FileName Node**:
* If `--file <filename>` is specified, the filename is embedded in the `Program` coordinate tuple as `(line col "filename")`. Any `FILE` token in the input is ignored.
* If `--file` is absent but a `FILE` token exists, the filename from the `FILE` token is embedded in the `Program` coordinate tuple.


2. **LCont**: `Body` always contains an `LCont` node as its last child. If `LCont` is absent in the source, a `Null` node is emitted inside `LCont`.
3. **SInfo Priority**: When `--file` is specified, `SInfo` is auto-inserted into `Body` unless an explicit `(__SInfo__ ...)` exists. Auto-inserted `SInfo` uses the location of the `Operator`.

---

### 5. Multi-File Output

When multiple source files are processed, each produces one `Program` node. They are written to standard output sequentially. A consumer must treat each top-level `(Program ...` as the start of a new AST tree.

---

### 6. Example

#### Source (`test.se`)

```scheme
(include "util.se")
(define main ,(args)
  __print__ "Hello")

```

#### Output: `parser --file test.se`

```scheme
(Program (1 1 "test.se")
  (Includer (1 1)
    (String (1 10) "util.se"))
  (Definition (2 2)
    (Variable (2 9) "main")
    (Function (2 14)
      (Head (2 15)
        (Variable (2 16) "args"))
      (Body (3 3)
        (SInfo (3 3) "test.se" "3" "3")
        (Operator (3 3)
          (Variable (3 3) "__print__"))
        (OList (3 13)
          (String (3 13) "Hello"))
        (LCont (3 20)
          (Null (3 20)))))))
```
