## Specification of `comment_remover.py`

### Overview

`comment_remover.py` is a single-pass filter that reads the token stream produced by `lexer2.py`, removes all comment-related tokens and `NEWLINE` tokens, and writes the remaining tokens to standard output in the same tab-separated format. Errors are written to standard error.

This component sits between the lexer and the parser in the processing pipeline:

```
lexer2.py  →  comment_remover.py  →  parser.py
```

---

### Invocation

```
python comment_remover.py [<lexer_output_file>]
```

If `<lexer_output_file>` is provided, the program reads from that file. If omitted, the program reads from standard input. The file is read in UTF-8 encoding. The output is written to standard output.

---

### Input Format

Each input line must be a tab-separated record of four fields, as produced by `lexer2.py`:

```
<line> TAB <col> TAB <kind> TAB <value>
```

Empty lines are ignored.

---

### Output Format

Each token that is not removed is written to standard output as a single line in the same four-field tab-separated format as the input:

```
<line> TAB <col> TAB <kind> TAB <value>
```

The `line` and `col` values of each token are preserved unchanged from the input.

---

### Removal Rules

The program processes the token stream left-to-right. The following constructs are removed. All other tokens are passed through unchanged.

#### 1. NEWLINE

Every token whose `kind` is `NEWLINE` is removed unconditionally.

#### 2. Line Comment

A line comment is the following token sequence:

```
LINE_COMMENT_BEGIN  [LINE_COMMENT_CONTENT]
```

Both tokens are removed. `LINE_COMMENT_CONTENT` is removed only if it immediately follows `LINE_COMMENT_BEGIN`; if the token after `LINE_COMMENT_BEGIN` has any other kind, only `LINE_COMMENT_BEGIN` is removed. The `NEWLINE` that follows a line comment is removed separately by Rule 1.

#### 3. Block Comment

A block comment is a nested token sequence that begins with `BLOCK_COMMENT_BEGIN` and ends with its matching `BLOCK_COMMENT_END`. The extent is determined by a nesting depth counter, initialised to 1 when the opening `BLOCK_COMMENT_BEGIN` is consumed:

- Each subsequent `BLOCK_COMMENT_BEGIN` increments the depth by 1.
- Each subsequent `BLOCK_COMMENT_END` decrements the depth by 1.
- When the depth reaches 0, the `BLOCK_COMMENT_END` that caused it is the closing delimiter, and the entire sequence from the opening `BLOCK_COMMENT_BEGIN` to this `BLOCK_COMMENT_END` inclusive is removed.

All tokens between the opening and closing delimiters — including nested `BLOCK_COMMENT_BEGIN`, `BLOCK_COMMENT_TEXT`, `BLOCK_COMMENT_END`, and any other token kind — are removed as part of the block comment.

If the token stream is exhausted before the depth reaches 0, the error message `"Error: Unterminated block comment"` is written to standard error.

If a `BLOCK_COMMENT_END` or `BLOCK_COMMENT_TEXT` token is encountered outside a block comment, the error message `"Error: Unexpected <kind> at <line>:<col>"` is written to standard error, and the token is skipped.

#### 4. S-Expression Comment

An S-expression comment is the following token sequence:

```
SEXP_COMMENT_BEGIN  <S-expression>
```

Both `SEXP_COMMENT_BEGIN` and the immediately following S-expression are removed. An S-expression is defined as one of:

- **Atom**: a single token whose `kind` is `SYMBOL` or `STRING`.
- **List**: a `LPAREN` token followed by any sequence of tokens (including nested lists) followed by the matching `RPAREN` token. The matching `RPAREN` is determined by a parenthesis depth counter, initialised to 1 when the `LPAREN` is consumed:
  - Each subsequent `LPAREN` increments the depth by 1.
  - Each subsequent `RPAREN` decrements the depth by 1.
  - When the depth reaches 0, the `RPAREN` that caused it is the closing delimiter.

If `SEXP_COMMENT_BEGIN` is not immediately followed by a token whose kind is `SYMBOL`, `STRING`, or `LPAREN`, the error message `"Error: SEXP_COMMENT_BEGIN at <line>:<col> not followed by a valid S-expression"` is written to standard error, and only `SEXP_COMMENT_BEGIN` is consumed.

---

### Token Kinds Passed Through

After removal, the only token kinds that may appear in the output are:

`SYMBOL`, `STRING`, `LPAREN`, `RPAREN`, `COMMA`

---

### Error Handling

All errors are written to standard error. The program does not halt on error; it skips the offending token and continues processing. A non-zero exit code is not guaranteed on error.
