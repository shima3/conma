## AST Output Specification

### 1. General Principles

* **Location information is always attached**: every node name is followed by `@line:col`, where `line` and `col` are the 1-based line and column numbers of the first character of the token that introduced the node.
* **Automatic SInfo insertion**: when the `--file <filename>` option is given, the parser inserts a `SInfo` node into every `Body` that does not already contain an explicit `(__SInfo__ ...)` construct.
* **Node Boundaries**: Non-terminal nodes are explicitly closed with a matching `End` tag to clearly define the tree structure.

---

### 2. Node Format

#### Terminal Nodes

```
NodeType@line:col [: value]

```

* Present for: `Variable`, `String`, `SInfo`, `FileName`, and `Null`.
* `: value`: For `SInfo`, the value is a space-separated sequence of string literals. For `Variable`, `String`, and `FileName`, the value is the exact text enclosed in double quotes.

#### Non-terminal Nodes

```
NodeType@line:col
  [Child Nodes...]
End@line:col: NodeType

```

* Child nodes are indented by two spaces relative to their parent.
* The `End` tag repeats the line:col and NodeType of the starting node.

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
* If `--file <filename>` is specified, a `FileName@0:0: "<filename>"` node is inserted as the first child of `Program`. Any `FILE` token in the input is ignored.
* If `--file` is absent but a `FILE` token exists, a `FileName` node is created using that token's value and position (`@line:col` of the `FILE` token).


2. **LCont**: `Body` always contains an `LCont` node as its last child. If `LCont` is absent in the source, a `Null` node is emitted inside `LCont`.
3. **SInfo Priority**: When `--file` is specified, `SInfo` is auto-inserted into `Body` unless an explicit `(__SInfo__ ...)` exists. Auto-inserted `SInfo` uses the location of the `Operator`.

---

### 5. Multi-File Output

When multiple source files are processed, each produces one `Program` node. They are written to standard output sequentially. A consumer must treat each line matching `Program@...` as the start of a new AST tree.

---

### 6. Example

#### Source (`test.se`)

```scheme
(include "util.se")
(define main ,(args)
  __print__ "Hello")

```

#### Output: `parser --file test.se`

```text
Program@1:1
  FileName@0:0: "test.se"
  Includer@1:1
    String@1:10: "util.se"
  End@1:1: Includer
  Definition@2:2
    Variable@2:9: main
    Function@2:14
      Head@2:15
        Variable@2:16: args
      End@2:15: Head
      Body@3:3
        SInfo@3:3: "test.se" "3" "3"
        Operator@3:3
          Variable@3:3: __print__
        End@3:3: Operator
        OList@3:13
          String@3:13: "Hello"
        End@3:13: OList
        LCont@3:20
          Null@3:20
        End@3:20: LCont
      End@3:3: Body
    End@2:14: Function
  End@2:2: Definition
End@1:1: Program

```
