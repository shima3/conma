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
1. If `--file <filename>` is specified: A `FileName` node is created using `<filename>`. Any `FILE` token in the input stream is ignored. Automatic `SInfo` insertion is enabled.
2. If `--file` is NOT specified but a `FILE` token exists: A `FileName` node is created using the value of the `FILE` token. Automatic `SInfo` insertion is NOT performed.
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

The AST is written to standard output as S-expressions. Each node is written as:

```
(NodeType (line col) children...)
```

where `line` and `col` are 1-based integers. Children are indented by 2 spaces. Consecutive closing parentheses on separate lines are merged onto the preceding line.

#### Terminal nodes

| Node | Format |
|---|---|
| `Variable` | `(Variable (line col) "name")` |
| `String` | `(String (line col) "value")` |
| `SInfo` | `(SInfo (line col) "file" "line" "col")` |
| `Null` | `(Null (line col))` |

The variable name in a `Variable` node is enclosed in double quotes.

#### Program node

The filename, when present, is embedded in the coordinate tuple as a third element:

```
(Program (line col "filename")
  children...)
```

If no filename is available, the coordinate tuple contains only two elements: `(line col)`.

`FileName` is not emitted as a separate node; it is folded into the `Program` coordinate tuple.

#### Non-terminal nodes

```
(NodeType (line col)
  children...)
```

Non-terminal nodes always emit a closing `)`, even when they have no children.



---

### Example

#### Source (`sample.se`)

```scheme
(define main ,(args)
  __print__ "Hello")

```

#### Output: `parser --file sample.se`

```scheme
(Program (1 1 "sample.se")
  (Definition (1 2)
    (Variable (1 9) "main")
    (Function (1 14)
      (Head (1 15)
        (Variable (1 16) "args"))
      (Body (2 3)
        (SInfo (2 3) "sample.se" "2" "3")
        (Operator (2 3)
          (Variable (2 3) "__print__"))
        (OList (2 13)
          (String (2 13) "Hello"))
        (LCont (2 20)
          (Null (2 20)))))))
```

---

### Error Handling

On a parse error, the error message is written to standard error and the program exits with code 1.
