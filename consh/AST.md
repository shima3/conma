# ConSh AST Output Specification

This document describes the syntax tree that `bin/consh_parse` writes for a ConSh script.
The grammar is the one in section 1.1 of [PLAN-ja.md](PLAN-ja.md).
The tree is the interface between the Python parser and the ConMa side of ConSh, which reads it and runs the script, so both sides follow this document.

---

## 1. Invocation

```
consh_parse [--pos] <script_file>
```

* Without `--pos`, the nodes carry no location information.
* With `--pos`, every node carries its location (see "Location Information").
* On a syntax error, `consh_parse` writes `consh_parse: <file>:<line>:<col>: <message>` to standard error and exits with status 1. The lines written before the error stay on standard output.

`consh_parse --expand` writes a different output, the result of word expansion; it is described at the top of `bin/consh_parse`.

---

## 2. General Principles

* **One line per command line**: the output has one S-expression per command line of the script, in order. A command line with no statement (an empty line, or a line with only a comment) is not written.
* **Comments are dropped**.
* **Line continuations are removed** before parsing (a `\` followed by a newline, outside `'...'` and comments). They leave no trace in the tree, except in the locations, which refer to the original text.
* **Node format**: a node is a list whose first element is a symbol, the node type, followed by its children. A list without a node type appears only as the list of names in `params`.
* **Strings**: names and texts are written in double quotes. In them, `\` is written as `\\`, `"` as `\"`, a newline as `\n`, a carriage return as `\r` and a tab as `\t`. Every other character, including non-ASCII characters, is written as it is.
* **Spacing**: the elements of a list are separated by one space. There are no other spaces and no line breaks inside a line.

---

## 3. Location Information

With `--pos`, every node has a coordinate tuple `(line col)` as its first child, right after the node type:

```scheme
(word (3 5) (lit (3 5) "abc"))
```

* `line` and `col` are 1-based numbers, written without quotes.
* `col` counts characters (Unicode code points) from the start of the line. A tab and a carriage return count as one character each.
* The location is that of the first character of the node in the original text. The table in "Node Kinds" gives, for each node, which character that is.
* The list of names in `params` has no coordinate tuple.

A reader tells the two forms apart by the first child: with `--pos` it is a list of two numbers, otherwise it is a string or a node.

---

## 4. Node Kinds

In the formats below, `[...]` is optional and `...` is a repetition of the element before it (zero or more times). Coordinate tuples are left out; with `--pos`, each node has one right after its type.

### Lines and Scripts

| Node | Format | Location |
| --- | --- | --- |
| `line` | `(line Statement ...)` (one or more statements) | the first statement |
| `script` | `(script line ...)` (zero or more lines) | the character right after `$(`, `{` or the keyword (`if`, `then`, `elif`, `else`, `do`) |

The top level of the output is a sequence of `line` nodes without an enclosing `script`.
A `script` appears as the body of a `function` and of a `fun`, in a `subst`, in the parts of an `if`, as the body of a `for`, of a `reset` and of a `shift`.

### Statements

| Node | Format | Grammar | Location |
| --- | --- | --- | --- |
| `return` | `(return word)` | `"return" スペース+ 語` | `return` |
| `break` | `(break ["name"])` | `"break" [スペース+ 変数名]` | `break` |
| `continue` | `(continue ["name"])` | `"continue" [スペース+ 変数名]` | `continue` |
| `export` | `(export Item ...)` (zero or more items) | 環境変数定義 | `export` |
| `function` | `(function "name" params script)` | 関数定義 | `function` |
| `assign` | `(assign def ...)` (one or more) | 変数定義 {スペース+ 変数定義} | the first `def` |
| `list` | `(list Element ...)` | コマンド列 | the first element |
| `bg` | `(bg Pipeline)` | a パイプライン followed by `&` | the pipeline |
| `pipe` | `(pipe Simple ...)` (two or more) | パイプライン | the first element |
| `cmd` | `(cmd [env] words [redirs])` | 単純文 with a コマンド | the first `def` of `env`, or else the first word |
| `if` | `(if clause ... [else] [redirs])` (one or more clauses) | 単純文 with an if文 | `if` |
| `for` | `(for "name" in script [redirs])` | 単純文 with a リストfor文 | `for` |
| `reset` | `(reset script [redirs])` | 単純文 with a reset文 | `reset` |
| `shift` | `(shift "name" [params] script)` | shift文 | `shift` |

A Statement is one of `return`, `break`, `continue`, `export`, `function`, `assign`, or a command list, which is written in the shortest form:

* A command list (コマンド列) is `(list Element ...)`, where an Element is `(bg Pipeline)` for a pipeline followed by `&`, or a Pipeline otherwise.
  If the command list has only one pipeline and no `&`, the `list` is left out and the Pipeline is written alone.
* A Pipeline (パイプライン) is `(pipe Simple ...)`, where a Simple is a `cmd`, an `if`, a `for`, a `reset` or a `shift`. If it has only one simple statement, the `pipe` is left out and the Simple is written alone.

Examples:

| Source | Statement |
| --- | --- |
| `f` | `(cmd ...)` |
| `f \| g` | `(pipe (cmd ...) (cmd ...))` |
| `f &` | `(list (bg (cmd ...)))` |
| `f & g \| h` | `(list (bg (cmd ...)) (pipe (cmd ...) (cmd ...)))` |
| `if f; then g; fi \| h` | `(pipe (if ...) (cmd ...))` |

### Parts of Statements

| Node | Format | Location |
| --- | --- | --- |
| `def` | `(def "name" [Value])` | the variable name |
| `array` | `(array word ...)` (zero or more) | `(` |
| `fun` | `(fun params script)` | `function` |
| `name` | `(name "name")` | the variable name |
| `env` | `(env def ...)` (one or more) | the first `def` |
| `words` | `(words word ...)` (one or more) | the first word |
| `redirs` | `(redirs redir ...)` (one or more) | the first `redir` |
| `redir` | `(redir "op" [Target])` | the first character of the operator |
| `params` | `(params ("name" ...) ["rest"])` | `(` |
| `clause` | `(clause script script)` | `if` or `elif` |
| `else` | `(else script)` | `else` |
| `in` | `(in word ...)` (zero or more) | `in` |

* `def` is a variable definition (`name=値`). A Value (値) is a `word`, an `array` for `(word ...)`, or a `fun` for a function value `function (a b . rest) { ... }`. The Value is left out when the right side is empty (`x=`).
* `fun` holds the variable list and the body of a function value. `function` is a function value only when `(` follows it, after spaces if any (`x=function` is a word).
* An Item of `export` is `(name "x")` for `export x`, or a `def` for `export x=値`.
* `env` holds the variable definitions before the command (`x=1 cmd`). It is left out when there are none.
* `redirs` holds the redirections after the words. It is left out when there are none.
* `redir`:
    * `op` is one of `"<"`, `">"`, `"2>"`, `">>"`, `"1>&2"`, `"2>&1"`.
    * The Target of `<`, `>` and `2>` is a `word` or a `params`. The Target of `>>` is a `word`. `1>&2` and `2>&1` have no Target.
* `params` is a variable list (変数リスト) `(a b . rest)`. The list of names may be empty, `()`. The rest name is left out when there is no `.`.
* `clause` is a part `if Script then Script` or `elif Script then Script` of an `if`: the first `script` is the condition, the second the body. The clauses are in the order of the source.
* `else` holds the script after `else`. It is left out when there is no `else`.
* Any of these scripts may be empty, `(script)`.
* The `redirs` of an `if` holds the redirections after `fi`.
* `for` holds the name of the loop variable, the words after `in` in an `in`, and the script between `do` and `done`. Its `redirs` holds the redirections after `done`.
* `break` and `continue` hold the name of a loop variable when one is given.
* `reset` holds the script between `{` and `}`, and its `redirs` the redirections after `}`. `reset` is a keyword only when `{` follows it, after spaces if any.
* `shift` holds the name of the variable for the continuation, the variable list for the values given to it (left out when there is none), and the script between `{` and `}`. A `shift` has no redirections. `shift` is a keyword only when spaces, a variable name, and `(` or `{` (after spaces if any) follow it.

### Words

| Node | Format | Location |
| --- | --- | --- |
| `word` | `(word WordElement ...)` (one or more) | the first element |
| `lit` | `(lit "text")` | the first character |
| `esc` | `(esc "c")` | `\` |
| `sq` | `(sq "text")` | `'` |
| `dq` | `(dq DqElement ...)` (zero or more) | `"` |
| `var` | `(var "name")` | `$` |
| `bvar` | `(bvar "name" [default])` | `$` |
| `default` | `(default [Value])` | `-` |
| `subst` | `(subst script)` | `$` |
| `glob` | `(glob word)` | `$` |

A WordElement (語要素) is one of the following:

* `lit`: a run of bare characters (裸文字). A `$` that does not start a variable reference or a command substitution is a bare character too. Adjacent bare characters are always in one `lit`, also across a removed line continuation.
* `esc`: `\` and the character after it outside quotes. The value is that character alone.
* `sq`: a string in single quotes, without the quotes. Nothing in it is processed.
  An ANSI-C quote `$'...'` is an `sq` too, with its escapes already turned into characters: `\n`, `\t`, `\r`, `\\`, `\'`, `\"`, `\xHH` (a byte; a run of them is read as UTF-8) and `\uHHHH`. A `\` before any other character is kept with that character.
* `dq`: a string in double quotes.
* `var`: a variable reference without braces, `$x`. The name is a variable name, one digit (`$1`), or one of `@`, `#`, `?`, `!`, `$`.
* `bvar`: a variable reference in braces, `${x}`. The name is a variable name, the digits of `${10}`, or `@`.
  `default` holds the default value of `${x-値}`; its Value is left out for `${x-}`. The `default` is left out when there is no `-`.
  The Value is read as a word also inside double quotes, so `"${x-"a b"}"` has a `dq` inside a `dq`.
* `subst`: a command substitution `$( ... )`.
* `glob`: `${-語}`, the LazyList of the names that the glob pattern `語` matches. The word is read as in the default value of `${x-値}`, also inside double quotes, and must not be empty.

A DqElement (二重引用要素) is `lit`, `var`, `bvar`, `subst` or `glob`:

* `lit` holds the text between the variable references and command substitutions, after `\` is processed: `\"`, `\$` and `\\` become `"`, `$` and `\`; a `\` before any other character is kept with that character (`\a` stays `\a`).
* Adjacent texts are in one `lit`.
* `""` is `(dq)`.

---

## 5. Example

Source (`example.sh`):

```sh
x=1 y=
FOO=bar f "a $x" > (a . rest) | g &   # comment
function h (s) { echo $(cat $s) }
```

Output of `consh_parse example.sh`:

```scheme
(line (assign (def "x" (word (lit "1"))) (def "y")))
(line (list (bg (pipe (cmd (env (def "FOO" (word (lit "bar")))) (words (word (lit "f")) (word (dq (lit "a ") (var "x")))) (redirs (redir ">" (params ("a") "rest")))) (cmd (words (word (lit "g"))))))))
(line (function "h" (params ("s")) (script (line (cmd (words (word (lit "echo")) (word (subst (script (line (cmd (words (word (lit "cat")) (word (var "s"))))))))))))))
```

Output of `consh_parse --pos example.sh` for the first line:

```scheme
(line (1 1) (assign (1 1) (def (1 1) "x" (word (1 3) (lit (1 3) "1"))) (def (1 5) "y")))
```
