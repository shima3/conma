# Self-hosted Front End

This document describes the plan to implement the front end of ConMa in ConMa itself: `includer`, `lexer`, `comment_remover`, `parser` and `resolver`.
It comes before the implementation of `eval`, which will use these components to turn source code into a module at run time.

The Python versions in `bin/` run as separate programs that pass text through files and pipes.
Converting between text and data at every stage would be slow, so the ConMa versions pass ConMa values to each other directly.
Text is produced only by the tests, to compare the result with the Python versions.

---

## Data Flow

```
includer
  for each file:  lexer → comment_remover → parser   (Seq of tokens between the stages)
  → LList of Programs
→ resolver
→ Module (LList)
```

* `lexer`, `comment_remover` and `parser` pass tokens to each other as a `Seq` (see "Sequence" in [SPEC.md](SPEC.html)).
* `parser` returns one `Program` as an LList.
* `resolver` takes an LList of one or more `Program`s and returns one module as an LList.

---

## Components

### lexer

Follows [LEXER.md](LEXER.html) and gives the same tokens and error messages as `bin/lexer` (`include/lexer.se`).

```
__Lexer_main__ fileName lines ,(tokens)
```

* `lines` is a Seq that passes the source one line at a time, each line with its newline (the last one may have none), as `__Stream_lines__` does. `tokens` is a Seq that passes the `FILE` token with `fileName` as its value, then the tokens of the source.
* The source is read line by line, so the following state is kept across lines:
  * the current line number (the column is counted from where the line starts);
  * the nesting depth of block comments;
  * the buffer of block comment text;
  * the line and column numbers where that text starts (a `BLOCK_COMMENT_TEXT` token may span several lines).
* The state is held by the Sink that `__Lexer_main__` passes to `lines` (the line Sink). Each line returns a new line Sink with the new state. At the end, the line Sink is applied to null: it writes the rest of the block comment text and returns the Sink of the tokens.
* `bin/lexer` reads the source in text mode, so `\r\n` and a lone `\r` become `\n` before it is scanned. The ConMa lexer treats them as newlines in the same way: a `NEWLINE` token always holds `"\n"`, and a newline inside a block comment is added to the text as `"\n"`.
* Outside a block comment, the first character selects a handler from a Dict (`__Lexer_handlers__`), and SymbolChars are looked up in another Dict, instead of comparing the character with each rule in turn. Lines are scanned one by one, so the String primitives only work on short Strings.
* Errors are written to standard error with the messages of `bin/lexer`, and scanning goes on.
* One difference: a `\` just before a newline inside a String. `bin/lexer` takes the newline into the String and goes on without counting the line (see Limitations in [LEXER.md](LEXER.html)). The ConMa lexer ends the String after the `\` and reports it as unterminated.
* Speed: with `test/vproc_run.py`, `include/util.se` (about 120 lines) takes about 3 seconds and `include/lexer.se` (about 360 lines) about 11 seconds.

### comment_remover

Follows [COMMENT_REMOVER.md](COMMENT_REMOVER.html) and gives the same tokens and error messages as `bin/comment_remover` (`include/comment_remover.se`).

```
__CommentRemover_main__ tokens ,(tokens)
```

* `tokens` is the Seq of Tokens that `__Lexer_main__` returns. The result is a Seq of the Tokens without comments and `NEWLINE`s, so the two can be chained: `__Lexer_main__ path lines ,(tokens) __CommentRemover_main__ tokens ,(tokens)`.
* It keeps the nesting depth of block comments and the parenthesis depth of an S-expression comment (`#;`) as its state. The state is the Sink passed to `tokens` (the state Sink), one for each of these states:
  * `normal`: outside comments;
  * `after_line`: just after `LINE_COMMENT_BEGIN`, to remove a `LINE_COMMENT_CONTENT`;
  * `block`: in a block comment, with its depth;
  * `after_sexp`: just after `SEXP_COMMENT_BEGIN`, with that token for the error message;
  * `sexp`: in the parentheses of an S-expression comment, with their depth.
* A state Sink takes a Token and returns the next state Sink. At the end it is applied to null: it reports a block comment or an S-expression comment start that is not finished, and returns the Sink of the output.
* As in `bin/comment_remover`, the token after `SEXP_COMMENT_BEGIN` must be `SYMBOL`, `STRING` or `LPAREN`; anything else, a `NEWLINE` included, is an error, and that token is processed as usual.

### parser

Follows [PARSER.md](PARSER.html) and gives the same AST and error messages as `bin/parser` (`include/parser.se`). It returns one `Program` as an LList.

```
__Parser_main__ onError fileName tokens ,(Program)
```

* `tokens` is the Seq of Tokens that `__CommentRemover_main__` returns.
* If `fileName` is not null, it works as `bin/parser --file <fileName>`, the mode `includer` uses: the position of the `Program` holds `fileName`, and a `Body` without an explicit `(__SInfo__ ...)` gets an `SInfo` with the position of its Operator. If `fileName` is null, the position of the `Program` holds the value of the `FILE` token, if there is one, and no `SInfo` is added.
* On a syntax error, the CChain is set back to what it was when `__Parser_main__` was called, and `onError` is applied to the message, which is the message of `bin/parser` without `SyntaxError: `. So if `onError` returns a value, it becomes the value of `__Parser_main__`, as with `onMissing` of `__PDict_lookup_or__`.
* The Tokens are collected into an LList first, and the parser is a recursive descent over it. A function for a rule returns the node and the rest of the LList; a function for an expression also returns the last Token it read, because a `Body` without an LCont puts its `LCont` just after that Token.
* Messages show values as Python's `repr` does: in single quotes, or in double quotes if the value has `'` and no `"`; `\`, the quote, newline, tab and carriage return are escaped.
* As in `bin/parser`, a `Body` without an LCont has an `LCont` with no children, `(LCont (line col))`. The example in [PARSER.md](PARSER.html) shows `(LCont (2 20) (Null (2 20)))` instead.
* A `String` node holds the value of the `STRING` token without its first and last characters, the double quotes. For a String that the lexer reports as unterminated, this drops its last character; `bin/parser` writes such a String without a closing quote, which is not a valid S-expression either.

### includer

Follows [INCLUDER.md](INCLUDER.html):

1. It takes one source file name and zero or more include directory names. It normalizes the source file name to an absolute path with `__OS_realpath__` and adds it to the file list.
2. For each file in the file list, in order, it runs `lexer`, `comment_remover` and `parser` and makes a `Program`.
3. Include statements are processed breadth first. For each include statement in the new `Program`:
   * An absolute path is normalized if it exists. Otherwise it is an error.
   * A relative path is searched for in the directory of the file that contains the include statement, then in each include directory in order. The first file found is normalized to an absolute path. If none is found, it is an error.
   * The normalized path is looked up in the file list. If it is already there (processed or not), a warning is written and the file is skipped. Otherwise it is added to the end of the file list.
4. It returns the `Program`s as an LList, in the order they were made.

The order matters: `resolver` numbers the global variables in the order of the `Program`s, so a different order would give a different module.

If a directory with the name given in an include statement exists in a directory that is searched (the directory of the file that contains the include statement, or an include directory), the behavior is undefined.

### resolver

Follows [RESOLVER.md](RESOLVER.html). It takes an LList of one or more `Program`s and returns the `Module`, with its `(F ...)` and `(G id "name" ...)` entries.
It annotates each `Variable` with `(L n)` or `(G n)` by making a new `Variable` node with `__AST_new__`, with the annotation added at the end of its children.

---

## Data Types

Tokens and AST nodes are passed between components, so their functions are named after the data type, not after a component.

| File | Functions | Used by |
|---|---|---|
| `include/token.se` | `__Token_new__`, `__Token_get_kind__`, `__Token_get_value__`, `__Token_get_line__`, `__Token_get_col__`, `__Token_toSexp__` | lexer, comment_remover, parser |
| `include/ast.se` | `__AST_new__`, `__AST_get_kind__`, `__AST_get_children__`, `__AST_get_pos__`, `__AST_toSexp__` | parser, resolver, includer |

### Tokens

| Function | Description |
|---|---|
| `__Token_new__ kind value line col ,(Token)` | Makes a Token. `kind` and `value` are Strings, `line` and `col` are Numbers. |
| `__Token_get_kind__ Token ,(String)` | The kind, such as `"SYMBOL"` (see [LEXER.md](LEXER.html)). |
| `__Token_get_value__ Token ,(String)` | The source text of the token. A `NEWLINE` token holds the newline itself, always `"\n"` (see the lexer above). |
| `__Token_get_line__ Token ,(Number)` / `__Token_get_col__ Token ,(Number)` | The position of the first character. The `FILE` token is at `0 0`. |
| `__Token_toSexp__ Token ,(String)` | `(Token (line col) KIND "value")`, for example `(Token (4 2) SYMBOL "define")`. |

* A Token is the LList `(kind value line col)`.
* `__Token_toSexp__` escapes `\`, `"`, newline, tab and carriage return in the value as String literals do (`\\`, `\"`, `\n`, `\t`, `\r`), because the value of a comment token or a `NEWLINE` can hold any of them. The value of a `STRING` token keeps its double quotes, so `"s\"x"` is written as `"\"s\\\"x\""`.

### AST nodes

| Function | Description |
|---|---|
| `__AST_new__ kind pos children ,(AST)` | Makes a node. `kind` is a String, `pos` is an LList or null, `children` is an LList or null. |
| `__AST_get_kind__ AST ,(String)` | The kind, such as `"Variable"` (see [AST.md](AST.html)). |
| `__AST_get_pos__ AST ,(pos)` | The position, or null. |
| `__AST_get_children__ AST ,(children)` | The children, or null if there are none. |
| `__AST_toSexp__ AST ,(String)` | The node as an S-expression on one line, such as `(Variable (4 26) "x" (L 0))`. |

* An AST node is the LList `(kind pos child ...)`: `__AST_new__` puts `kind` and `pos` in front of `children`. `__AST_toSexp__` recognizes a node by `__type_name__` returning `"LList"`, so a node cannot be a Closure.
* The position of a node is a list of two Numbers, `(line col)`. The position of a `Program` has three elements, two Numbers followed by the file name String: `(1 1 "s.se")`. `__AST_get_pos__` and `__AST_toSexp__` handle both.
* Some nodes have no position (`Module`, `(F ...)`, `(G ...)`, `(L n)`). `__AST_new__` accepts null for the position, and `__AST_toSexp__` writes no position for it.
* A node with neither a position nor children is written as its kind alone. This is how `resolver` writes an undefined global variable: `(G 1 "g" Undefined)`.
* The children of a node are an LList of Strings (such as `"name"`), Numbers and AST nodes. `__AST_toSexp__` writes:
  * a String in double quotes, without escaping (see the next item);
  * a Number converted with `__Number_toString__`, without quotes;
  * an AST node recursively.

  Any other value stops the program with an error.
* A `String` node holds the source text of the literal without the surrounding double quotes. Escape sequences are kept as they are in the source, so `"a\"b\n"` is held as `a\"b\n`.
* An `SInfo` node holds its position as Numbers, but the file name, line and column after it as Strings: `(SInfo (2 3) "s.se" "2" "3")`.

---

## New Primitives

Two primitives are added. See Step 16 in [ROADMAP.md](ROADMAP.html).

### `__OS_realpath__ onError path ,(String)`

* Returns the absolute path of `path` with `..`, `.` and symbolic links resolved.
* A relative path is relative to the current directory (see `__OS_chdir__`).
* If the path does not exist, `onError` is invoked with an error message String.
* `includer` uses it to check that a file exists and to find duplicate files.

### `__type_name__ Value ,(String)`

Returns the name of the type of `Value`. A type is tested by comparing the name with `__String_equal__`:

```
__type_name__ child ,(t)
__String_equal__ t "String" ,(isString)
__if_then_else__ isString ...
```

| Value | Result |
|---|---|
| null (including an empty LList and an empty CChain) | `"null"` |
| a String (a literal or one made at run time) | `"String"` |
| a Number | `"Number"` |
| a Closure or a primitive function | `"Closure"` |
| a Ref, CFrame, LList, MSeq, Dict, VProc or OSProc | `"Ref"`, `"CFrame"`, `"LList"`, `"MSeq"`, `"Dict"`, `"VProc"`, `"OSProc"` |
| a Stream | `"Stream"` (the same for input and output Streams) |

Limitations:

* A Boolean, a Seq and a Sink are all Closures, so they give `"Closure"`.
* A CChain is an LList of CFrames, so it gives `"LList"`.

---

## Naming

All names have the reserved form `__name__`, because the components will become standard definitions used by `eval`, and ConMa has a single global namespace.

| Component | File | Entry point | Prefix of helpers |
|---|---|---|---|
| lexer | `include/lexer.se` | `__Lexer_main__` | `__Lexer_…__` |
| comment_remover | `include/comment_remover.se` | `__CommentRemover_main__` | `__CommentRemover_…__` |
| parser | `include/parser.se` | `__Parser_main__` | `__Parser_…__` |
| resolver | `include/resolver.se` | `__Resolver_main__` | `__Resolver_…__` |
| includer | `include/includer.se` | `__Includer_main__` | `__Includer_…__` |

---

## Tests

The result of each component is converted to an S-expression with `__Token_toSexp__` or `__AST_toSexp__` and compared with the output of the Python version:

| Component | Compared with |
|---|---|
| lexer | `bin/lexer <file>` |
| comment_remover | `bin/lexer <file> \| bin/comment_remover` |
| parser | `bin/lexer <file> \| bin/comment_remover \| bin/parser --file <file>` |
| includer | `bin/includer <file>` |
| resolver | `bin/includer <file> \| bin/resolver` |

### Comparing outputs

`test/selfhost_compare.py` reads both outputs and compares them as data, so the ConMa side does not need to reproduce the layout of the Python versions:

```
test/selfhost_compare.py tokens <python_output> <conma_output>
test/selfhost_compare.py ast    <python_output> <conma_output>
```

* `tokens` compares the tab-separated output of `bin/lexer` or `bin/comment_remover` with one `(Token (line col) KIND "value")` per line, as `__Token_toSexp__` writes it. Both sides become lists of `(line, col, kind, value)` with the real characters in the value:
  * In the Python output, a line that does not start with `line TAB col TAB KIND TAB` continues the value of the previous token, because `bin/lexer` writes a `BLOCK_COMMENT_TEXT` that spans several lines as it is. The `NEWLINE` values `\n` and `\r\n` become the newline characters.
  * In the ConMa output, the escape sequences of the value are processed.
* `ast` compares two sequences of S-expressions, such as the output of `bin/parser`, `bin/includer` or `bin/resolver` with what `__AST_toSexp__` writes. Spaces and newlines between items are ignored. Strings are compared as source text, with their quotes and escape sequences, because both sides keep them that way.
* A file name `-` reads standard input.
* The exit code is 0 when the outputs are equal, 1 when they differ, and 2 for an output that cannot be read or a wrong command line. For a difference it prints the first one: the number of the token and both tokens, or the path to the differing item (such as `item 1 > Module > G 0 "f" > Function (6 11) > ...`) and both items.

`test/selfhostN` holds the tests of step N of the implementation order below. Each `test.sh` runs its cases like the tests of [ROADMAP.md](ROADMAP.html) and prints only the differences from the expected logs:

```
for t in selfhost1 selfhost2 selfhost3 selfhost4 selfhost5; do test/$t/test.sh; done
```

`test/selfhost1` tests `token.se` and `ast.se`:

| Case | What it checks |
|---|---|
| case1 | `__Token_new__` and the four getters |
| case2 | `__Token_toSexp__`: the `FILE` token, a `STRING` token, `NEWLINE` tokens, comment text with quotes, backslashes, newlines, tabs and Japanese text, an empty value |
| case3 | `__AST_new__` and the getters, also for a node with no position and no children |
| case4 | `__AST_toSexp__`: a `String` node is not escaped again; a `Variable` with `(L 0)`; `SInfo`; a node with no children; nested nodes |
| case5 | the three-element position of a `Program`; `Module`, `F` and `G` with no position; `Undefined` |
| case6 | a child that is not a String, Number or node stops the program with an error |

`bin/includer` prints a warning (`circular include ignored`) for `util.se`, because both `token.se` and `ast.se` include it. The warning is expected and does not change the result.

`test/selfhost2` tests `selfhost_compare.py`. `test.sh` runs the Python tools on `sample.se` (which includes `inc.se`); `conma/*.txt` holds the same outputs in the ConMa format. Absolute paths in the outputs of `bin/includer` and `bin/resolver` are written as `<dir>`. The differences are made by changing a copy of `conma/*.txt` with `sed`.

| Case | What it checks |
|---|---|
| case1 | equal tokens for `bin/lexer` and `bin/comment_remover`: comments with quotes, backslashes, tabs and Japanese text, a nested block comment over two lines, an S-expression comment; standard input |
| case2 | equal S-expressions for `bin/parser`, `bin/includer` and `bin/resolver`, one line per S-expression on the ConMa side |
| case3 | token differences: a value, an escape sequence, a column, a kind, a missing and an extra token |
| case4 | S-expression differences: a String, an atom (`G` and `L`), a missing and an extra child, a missing and an extra S-expression |
| case5 | exit code 2: a line that is not a Token, a line that is not a TSV token, unbalanced parentheses, a missing file, an unknown mode |

`bin/lexer` reads the source in text mode, so a `\r\n` line ending becomes `\n` before it is scanned: `inc.se` ends with `\r\n`, but its `NEWLINE` token is `\n`. The ConMa lexer has to do the same to give the same tokens.

`test/selfhost3` tests the lexer. `test.se` is a program for `vproc_run.py --main` that lexes each file given as an argument and writes the tokens with `__Token_toSexp__`. For each file, `test.sh` compares the tokens with `bin/lexer` by `selfhost_compare.py tokens`, and the error messages with `diff`, which prints nothing when they are the same.

| Case | What it checks |
|---|---|
| case1 | `selfhost2/sample.se` and `selfhost2/inc.se`: comments, strings, Japanese text, a `\r\n` line ending |
| case2 | `edge.se`: `\|#` after a Symbol and outside a block comment, `\|` as a Symbol, unexpected characters (`#`, `@`, `` ` ``, `\`, Japanese text), escapes, an empty String and an empty line comment, an unterminated String, a lone `\r` and `\r\n`, block comments one after another and nested over two lines, no newline at the end |
| case3 | `open.se`: a block comment that is not closed at the end; `empty.se`: an empty file |
| case4 | real sources: `include/util.se` and `test/step14/test.se` |
| case5 | `backslash.se`: the difference above; `selfhost_compare.py` reports the String and exits with 1 |

`test/selfhost4` tests the comment remover in the same way. `test.se` lexes each file, removes the comments and writes the tokens; it is compared with `bin/lexer <file> | bin/comment_remover`. The ConMa lexer and comment remover run together, so their error messages are mixed: both sides are sorted before `diff`, and the log shows them. An argument `=tokens` runs a Seq of Tokens made by hand instead of a file.

| Case | What it checks |
|---|---|
| case1 | `sexp.se`: S-expression comments of an atom, a String and a nested list; errors for a newline, another `#;` and `)` after `#;`; line comments with and without text |
| case2 | `block.se`: an unmatched `\|#`; block comments that hold `#;`, `(` and nested comments over several lines, also between tokens without spaces |
| case3 | the end of the source: an S-expression comment that is not closed (no error), `#;` at the end (error), a block comment that is not closed (`selfhost3/open.se`) |
| case4 | `selfhost2/sample.se`, `selfhost3/edge.se` and `include/util.se` |
| case5 | `BLOCK_COMMENT_TEXT` and `BLOCK_COMMENT_END` outside a block comment, which the lexer does not give |

`test/selfhost5` tests the parser. `test.se` takes a mode and files: `file` passes the file name to `__Parser_main__`, `nofile` passes null, and `recover` passes an `onError` that returns a value. In the modes `file` and `nofile`, a syntax error is written as `SyntaxError: <message>` and ends the program with exit code 1, as `bin/parser` does. `test.sh` compares each file in both modes with `bin/lexer <file> | bin/comment_remover | bin/parser [--file <file>]`: the exit codes, the AST by `selfhost_compare.py ast`, and the error messages. The whole test takes about 40 seconds.

| Case | What it checks |
|---|---|
| case1 | `selfhost2/sample.se`, `selfhost2/inc.se` and `include/util.se` |
| case2 | `body.se`: explicit `SInfo` with and without Strings, an empty Body, a Body with only an LCont, a FuncExp as Operator and as the last operand, nested functions, a String with escapes, an Includer after a Definition |
| case3 | no statements: an empty file and a file with only a comment |
| case4 | syntax errors at the top level: no `(`, an unknown keyword, no Symbol after `(`, `(` alone, an Includer without a String |
| case5 | syntax errors in a Definition: no `,`, a String as a parameter (its `repr` has `\'`), a String as Operator, the end of input |
| case6 | `recover`: the value of `onError` is the value of `__Parser_main__`, also for an error deep in the recursion, and the program goes on after it |

---

## Implementation Order

0. Add the primitives `__type_name__` and `__OS_realpath__`, with documentation and tests (Step 16 in [ROADMAP.md](ROADMAP.html)).
1. Write `include/token.se` and `include/ast.se`.
2. Write the Python script that compares the outputs of the Python and ConMa versions (`test/selfhost_compare.py`).
3. lexer
4. comment_remover
5. parser
6. includer
7. resolver

Each component is tested with the output of the ConMa versions of the components before it.
