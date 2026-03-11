## Specification of `parser.py`

### Overview

`parser.py` reads the token stream produced by `comment_remover.py`, constructs an AST for one ConMa source file, and writes the AST to standard output per `AST.md`. Parse errors are written to standard error.

This component is the third stage of the processing pipeline:

```
lexer2.py  →  comment_remover.py  →  parser.py
```

---

### Invocation

```
parser.py [--file <filename>] [<token_file>]
```

- `--file <filename>`: when specified, `<filename>` is used for automatic `SourceInfo` insertion and is printed as a suffix on the `Program` node. If omitted, neither insertion nor suffix occurs.
- `<token_file>`: token stream to parse. If omitted, the program reads from standard input.

The file is read in UTF-8 encoding.

---

### Input Format

Each input line must be a tab-separated record of four fields, as produced by `comment_remover.py`:

```
<line> TAB <col> TAB <kind> TAB <value>
```

Empty lines are ignored. Lines with fewer than four fields are ignored.

The only token kinds expected in the input are: `SYMBOL`, `STRING`, `LPAREN`, `RPAREN`, `COMMA`. Any other kind causes a parse error at the point it is encountered.

---

### Output Format

The AST is written to standard output per `AST.md`. On a parse error, the error message is written to standard error and the program exits with code 1.

---

### Grammar

The parser implements the following grammar (from `SPEC.md`):

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

### Parsing Rules

#### Program

`Program` consumes `Statement`s until the token stream is exhausted. The location of the `Program` node is that of the first token; if the stream is empty, line 1, column 1 is used.

#### Statement

A `Statement` begins with `LPAREN`. The token immediately following determines the kind:

- `SYMBOL` `include` → `Includer`
- `SYMBOL` `define` → `Definition`
- Any other value → parse error

#### Includer

```
LPAREN  SYMBOL("include")  STRING  RPAREN
```

The `Includer` node's location is that of the `LPAREN`. Its sole child is a `String` node carrying the exact source text of the `STRING` token including its quotes.

#### Definition

```
LPAREN  SYMBOL("define")  SYMBOL  Function  RPAREN
```

The `Definition` node's location is that of the `SYMBOL("define")` token (not the `LPAREN`). Its children are `[Variable, Function]`.

#### Function

```
COMMA  Head  Body
```

The `Function` node's location is that of the `COMMA` token.

#### Head

```
LPAREN  { SYMBOL }  RPAREN
```

The `Head` node's location is that of the `LPAREN`. Each `SYMBOL` inside becomes a `Variable` child. If no parameters are present, `Head` has no children and its location is still that of the `LPAREN`.

#### Body

```
[ SourceInfo ]  Operator  OList  [ LCont ]
```

**Location**: if an explicit `SourceInfo` is present, the `Body` node's location is that of the `SourceInfo`'s `LPAREN`. Otherwise, the location is that of the `Operator`.

**SourceInfo detection**: the parser looks ahead two tokens. If the next token is `LPAREN` and the token after that is `SYMBOL("__SI__")`, the following construct is parsed as `SourceInfo`. Otherwise no `SourceInfo` is present in the source.

**Automatic SourceInfo insertion**: if no explicit `SourceInfo` is present in the source and `--file <filename>` was specified, the parser inserts an automatic `SourceInfo` node as the first child of `Body`. Its location and the embedded line and column values are those of the `Operator`. Its value is:

```
"<filename>" "<op_line>" "<op_col>"
```

where `op_line` and `op_col` are the numeric location of the `Operator`, written as quoted string literals.

If an explicit `SourceInfo` is present, the `--file` option does not override it.

**OList**: the parser consumes `Expression`s as long as the next token can start an `Expression` (`SYMBOL`, `STRING`, or `LPAREN`). It stops on `COMMA` (start of `LCont`), `RPAREN`, or end of stream.

**OList location**: the location of the first `Expression` in `OList`. If `OList` is empty, the location is that of the next token in the stream.

**LCont**: if the next token after `OList` is `COMMA`, a `Function` is parsed and wrapped in an `LCont` node. The `LCont` node's location is that of the `Function`.

**Absent LCont**: if no `COMMA` follows `OList`, a `Null` node is emitted inside `LCont`. The `Null` node's location is the column immediately after the last token of `OList` (i.e., `last_token.col + len(last_token.value)`), on the same line as that token. If `OList` is empty, the `Null` node's location is that of the next token in the stream.

**Body children** (in order):
1. `SourceInfo` (explicit or auto-inserted; omitted if neither)
2. `Operator`
3. `OList`
4. `LCont`

#### SourceInfo

```
LPAREN  SYMBOL("__SI__")  { STRING }  RPAREN
```

The `SourceInfo` node's location is that of the `LPAREN`. Its value is the space-joined sequence of the exact source text of all `STRING` tokens inside.

#### Operator

`Operator` wraps a single `Variable` (parsed from `SYMBOL`) or `FuncExp` node. Its location is that of its child.

#### Expression

`Expression` produces one of:
- `Variable`: from a `SYMBOL` token.
- `String`: from a `STRING` token. The value is the exact source text including quotes.
- `FuncExp`: from `LPAREN Function RPAREN`.

#### FuncExp

```
LPAREN  Function  RPAREN
```

The `FuncExp` node's location is that of the `LPAREN`.

---

### Error Handling

On any parse error, the message `SyntaxError: <description>` is written to standard error and the program exits with code 1. No partial AST is written.

Error messages include the location `line:col` of the offending token where available.
