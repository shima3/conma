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

Follows [LEXER.md](LEXER.html). The source is read in pieces, so the following state is kept across them:

* the current line and column numbers;
* the nesting depth of block comments;
* the buffer of block comment text;
* the line and column numbers where that text starts (a `BLOCK_COMMENT_TEXT` token may span several lines).

### comment_remover

Follows [COMMENT_REMOVER.md](COMMENT_REMOVER.html). It keeps the nesting depth of block comments and the parenthesis depth of an S-expression comment (`#;`) as its state.

### parser

Follows [PARSER.md](PARSER.html) with automatic SInfo insertion, the mode `includer` uses (`parser --file <file>`). It returns one `Program` as an LList.

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
| `__Token_get_value__ Token ,(String)` | The source text of the token. A `NEWLINE` token holds the newline itself (`"\n"` or `"\r\n"`). |
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

A Python script reads both outputs and compares them as data, so the ConMa side does not need to reproduce the indentation of the Python versions.

`test/selfhostN` holds the tests of step N of the implementation order below. Each `test.sh` runs its cases like the tests of [ROADMAP.md](ROADMAP.html) and prints only the differences from the expected logs:

```
for t in selfhost1; do test/$t/test.sh; done
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

---

## Implementation Order

0. Add the primitives `__type_name__` and `__OS_realpath__`, with documentation and tests (Step 16 in [ROADMAP.md](ROADMAP.html)).
1. Write `include/token.se` and `include/ast.se`.
2. Write the Python script that compares the outputs of the Python and ConMa versions.
3. lexer
4. comment_remover
5. parser
6. includer
7. resolver

Each component is tested with the output of the ConMa versions of the components before it.
