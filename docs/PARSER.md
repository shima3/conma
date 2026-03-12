## Specification of `parser`

### Overview

`parser` reads the token stream produced by `comment_remover`, constructs an AST for one ConMa source file, and writes the AST to standard output. Parse errors are written to standard error.

This component is the third stage of the processing pipeline:

```
lexer  →  comment_remover  →  parser

```

---

### Invocation

```
parser [--file <filename>] [<token_file>]

```

* **FileName Node Generation & Priority**:
1. If `--file <filename>` is specified: A `FileName` node is created using `<filename>`. Any `FILE` token in the input stream is ignored. Automatic `SourceInfo` insertion is enabled.
2. If `--file` is NOT specified but a `FILE` token exists: A `FileName` node is created using the value of the `FILE` token. Automatic `SourceInfo` insertion is NOT performed.
3. If neither is present: No `FileName` node is generated.


* `<token_file>`: token stream to parse. If omitted, the program reads from standard input.

---

### Input Format

Each input line must be a tab-separated record of four fields:

```
<line> TAB <col> TAB <kind> TAB <value>

```

The expected token kinds are: `FILE`, `SYMBOL`, `STRING`, `LPAREN`, `RPAREN`, `COMMA`.

---

### Output Format

The AST is written to standard output. Each node follows the format `NodeType@line:col`.

* **Terminal Nodes**: `Variable`, `String`, `SourceInfo`, `FileName`, and `Null` are printed on a single line (with values where applicable). `FileName` nodes generated via `--file` use `@0:0` as their location. `FileName` nodes generated from a `FILE` token use the line and column of that token.
* **Non-terminal Nodes**: These consist of a start line, indented children (2 spaces), and a matching **End tag**:
```
NodeType@line:col
  [Child Nodes...]
End@line:col: NodeType

```



---

### Example

#### Source (`sample.se`)

```scheme
(define main ,(args)
  __print__ "Hello")

```

#### Output: `parser --file sample.se`

```text
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

---

### Error Handling

On a parse error, the error message is written to standard error and the program exits with code 1.
