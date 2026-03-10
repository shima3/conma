## Specification of `lexer.py`

### Overview

`lexer.py` is a single-pass lexical analyzer for ConMa source files. It reads a source file, segments it into tokens, and writes each token to standard output in tab-separated format. Lexical errors are written to standard error.

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
- **kind**: one of `LPAREN`, `RPAREN`, `COMMA`, `STRING`, `SYMBOL`.
- **value**: the exact source text of the token.

---

### Scanning Rules

The scanner processes the source left-to-right, one character at a time. At each position, the following rules are applied in the order listed. The first matching rule is applied.

#### 1. Block Comment
Triggered by the two-character sequence `#|`.

The scanner enters block-comment mode and increments a nesting depth counter to 1. It advances character by character:
- On `#|`: increment depth by 1, advance 2.
- On `|#`: decrement depth by 1, advance 2. If depth reaches 0, block-comment mode ends.
- On `\n`: increment line number, update line-start position, advance 1.
- Otherwise: advance 1.

If the source is exhausted before depth reaches 0, an unterminated-block-comment error is written to standard error.

No token is emitted.

#### 2. Line Comment
Triggered by `;`.

The scanner advances until a `\n` character or end-of-source is reached. No token is emitted.

#### 3. Whitespace
Triggered by space (U+0020), horizontal tab (U+0009), or carriage return (U+000D).

The scanner advances by 1. No token is emitted.

#### 4. Newline
Triggered by `\n` (U+000A).

The scanner increments the line number and sets the line-start position to the character immediately following the newline. Advances by 1. No token is emitted.

#### 5. String
Triggered by `"`.

The scanner advances past the opening `"` and consumes characters until a closing `"` is found, observing the following escape rule: if `\` is encountered, the next character is consumed unconditionally (i.e., `\"` does not close the string). A `STRING` token is emitted containing the full source text including both delimiters.

#### 6. Single-Character Punctuation
- `(` → `LPAREN`
- `)` → `RPAREN`
- `,` → `COMMA`

The scanner advances by 1 and emits the corresponding token.

#### 7. Symbol
Triggered by any character in the set:

```
a–z  A–Z  0–9  _  !  ?  *  +  -  /  =  <  >  $  %  &  |  .  ^
```

The scanner consumes the maximal contiguous sequence of characters from this set. A `SYMBOL` token is emitted.

#### 8. Mismatch
Any character not matched by rules 1–7 is an error. The offending character, its line number, and its column number are written to standard error. The scanner advances by 1 and continues.

---

### Line and Column Tracking

- The line number starts at 1 and increments each time a `\n` is consumed (in rules 4 and inside block comments).
- The line-start position is the source index of the first character of the current line. The column of a token is `(token_start_index − line_start_index) + 1`.
- Block comments that span multiple lines update both the line number and the line-start position correctly.

---

### Limitations

- Block comment nesting is supported; however, unterminated block comments produce an error message but do not halt the scanner — the remainder of the source after the unterminated comment is not scanned.
- Strings that span multiple lines are not supported: a `\n` inside a string is consumed as a literal character without updating the line number, resulting in incorrect line and column numbers for all subsequent tokens.
- The `SourceInfo` construct (`__SI__`) is not treated specially; it is recognized as an ordinary `SYMBOL`.
