## Specification of `lexer.py`

### Overview

`lexer.py` is a single-pass lexical analyzer for ConMa source files. It reads a source file, segments it into tokens, and writes each token to standard output in tab-separated format. All tokens — including comment-related tokens and newlines — are preserved in the output. Comment removal and newline removal are performed by a downstream `comment_remover.py`. Lexical errors are written to standard error.

---

### Invocation

```
python lexer.py <source_file>
```

The program accepts exactly one command-line argument: the path to the source file. The file is read in UTF-8 encoding. If no argument is provided, a usage message is printed and the program exits.

---

### Output Format

Each recognized token is printed to standard output as a single line of four tab-separated fields:

```
<line> TAB <column> TAB <kind> TAB <value>
```

- **line**: 1-based line number of the first character of the token.
- **column**: 1-based column number of the first character of the token, measured in characters from the start of the current line.
- **kind**: one of the token kinds listed below.
- **value**: the exact source text of the token, except for `NEWLINE` (see below).

#### Token Kinds

| Kind                  | Value                                              |
|-----------------------|----------------------------------------------------|
| `SYMBOL`              | Maximal sequence of SymbolChars                    |
| `STRING`              | `"..."` including delimiters and escape sequences  |
| `LPAREN`              | `(`                                                |
| `RPAREN`              | `)`                                                |
| `COMMA`               | `,`                                                |
| `NEWLINE`             | `\n` or `\r\n` (written as escape sequence)        |
| `LINE_COMMENT_BEGIN`  | `;`                                                |
| `LINE_COMMENT_CONTENT`| Text from after `;` up to (not including) newline  |
| `BLOCK_COMMENT_BEGIN` | `#\|`                                              |
| `BLOCK_COMMENT_TEXT`  | Text inside a block comment (excludes `#\|`, `\|#`)|
| `BLOCK_COMMENT_END`   | `\|#`                                              |
| `SEXP_COMMENT_BEGIN`  | `#;`                                               |

`LINE_COMMENT_CONTENT` is emitted only when the content is non-empty.
`BLOCK_COMMENT_TEXT` is emitted only when the content is non-empty.
`NEWLINE` value is written as the two-character escape sequence `\n` or `\r\n` so that the output remains valid line-oriented TSV.

---

### Scanning Rules

The scanner processes the source left-to-right, one character at a time. At each position, the following rules are applied in the order listed. The first matching rule is applied. Two-character sequences are always checked before single-character rules.

#### Outside a block comment

**1. `#|` — Block Comment Begin**
Emits `BLOCK_COMMENT_BEGIN`. Increments the block-comment nesting depth to 1. The scanner enters block-comment mode. Advances 2.

**2. `#;` — S-Expression Comment Begin**
Emits `SEXP_COMMENT_BEGIN`. Advances 2.

**3. `|#` — Block Comment End (unmatched)**
Emits `BLOCK_COMMENT_END`. Advances 2.
Note: a `|#` appearing outside a block comment is anomalous; `comment_remover.py` reports it as an error.

**4. `\r\n` — Windows Newline**
Emits `NEWLINE` with value `\r\n`. Increments the line number and updates the line-start position. Advances 2.

**5. `\n` — Unix Newline**
Emits `NEWLINE` with value `\n`. Increments the line number and updates the line-start position. Advances 1.

**6. Space, Tab, isolated CR — Whitespace**
Consumed silently. Advances 1. No token is emitted.

**7. `;` — Line Comment Begin**
Emits `LINE_COMMENT_BEGIN`. Then advances character by character until `\n`, `\r\n`, or end-of-source is reached. If any characters were consumed, emits a single `LINE_COMMENT_CONTENT` token containing all of them. The terminating newline is not consumed here; it is emitted as a subsequent `NEWLINE` token by rule 4 or 5.

**8. `(` `)` `,` — Single-Character Punctuation**
Emits `LPAREN`, `RPAREN`, or `COMMA` respectively. Advances 1.

**9. `"` — String**
Advances past the opening `"` and consumes characters until a closing `"` is found. If `\` is encountered, the next character is consumed unconditionally (`\"` does not close the string). Emits a `STRING` token containing the full source text including both delimiters. If `\n` is encountered before a closing `"`, an unterminated-string error is written to standard error and scanning resumes at the `\n`.

**10. SymbolChar — Symbol**
Triggered by any character in the set:
```
a–z  A–Z  0–9  _  !  ?  *  +  -  /  =  <  >  $  %  &  |  .  ^
```
The scanner consumes the maximal contiguous sequence of SymbolChars, stopping immediately before any `|#` two-character sequence. Emits a `SYMBOL` token.
Note: `|` is a SymbolChar, but the sequence `|#` is always tokenized as `BLOCK_COMMENT_END` and is never consumed as part of a Symbol.

**11. Mismatch**
Any character not matched by rules 1–10 is an error. The offending character, its line number, and its column number are written to standard error. The scanner advances by 1 and continues.

#### Inside a block comment (nesting depth ≥ 1)

Characters are accumulated into a `BLOCK_COMMENT_TEXT` buffer until a `#|` or `|#` sequence is encountered, at which point the buffer is flushed as a `BLOCK_COMMENT_TEXT` token (if non-empty).

- On `#|`: flush buffer, emit `BLOCK_COMMENT_BEGIN`, increment depth, advance 2.
- On `|#`: flush buffer, emit `BLOCK_COMMENT_END`, decrement depth. If depth reaches 0, block-comment mode ends. Advance 2.
- On `\r\n`: append `\r\n` to buffer, increment line number, update line-start position, advance 2.
- On `\n`: append `\n` to buffer, increment line number, update line-start position, advance 1.
- Otherwise: append character to buffer, advance 1.

If the source is exhausted before depth reaches 0, any remaining buffer content is flushed and an unterminated-block-comment error is written to standard error.

---

### Line and Column Tracking

- The line number starts at 1 and is incremented each time `\n` or `\r\n` is consumed, both outside and inside block comments.
- The line-start position is the source index of the first character of the current line. The column of a token is `(token_start_index − line_start_index) + 1`, giving a 1-based column number.
- `BLOCK_COMMENT_TEXT` tokens that span multiple lines carry the line and column of their first character.

---

### Limitations

- Strings that span multiple lines are not supported: a `\n` inside a string is consumed as a literal character without updating the line number, resulting in incorrect line and column numbers for all subsequent tokens.
- The `SourceInfo` construct (`__SI__`) is not treated specially; it is recognized as an ordinary `SYMBOL`.
