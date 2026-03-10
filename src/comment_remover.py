"""
comment_remover.py -- ConMa comment remover

Reads lexer2 output (tab-separated: line TAB col TAB kind TAB value)
from a file or stdin, removes all comment tokens and NEWLINE tokens,
and writes the remaining tokens to stdout in the same format.

Removed constructs (per SPEC.md "Comment Removing"):

  LineComment:
    LINE_COMMENT_BEGIN  LINE_COMMENT_CONTENT?
    (the following NEWLINE is also removed as part of NEWLINE removal)

  BlockComment (nested):
    BLOCK_COMMENT_BEGIN
      ( BlockComment | BLOCK_COMMENT_TEXT )*
    BLOCK_COMMENT_END

  S-Expression Comment:
    SEXP_COMMENT_BEGIN  <one S-expression>

    An S-expression is one of:
      - A single SYMBOL or STRING token (atom)
      - A parenthesised list: LPAREN ... RPAREN (balanced, any content)

  NEWLINE:
    All NEWLINE tokens are removed (not used by the parser).

Usage:
    python comment_remover.py [<lexer_output_file>]
    python lexer2.py source.se | python comment_remover.py
"""

import sys


def read_tokens(lines):
    tokens = []
    for raw in lines:
        raw = raw.rstrip('\n')
        if not raw:
            continue
        parts = raw.split('\t', 3)
        if len(parts) == 4:
            lnum, col, kind, value = parts
            tokens.append((int(lnum), int(col), kind, value))
    return tokens


def remove_comments(tokens):
    """Return a new token list with all comment constructs and NEWLINEs removed."""
    out = []
    i = 0
    n = len(tokens)

    def skip_sexp(pos):
        """
        Skip one S-expression starting at pos.
        Returns the index of the first token AFTER the S-expression,
        or pos if no valid S-expression is found.
        """
        if pos >= n:
            return pos
        kind = tokens[pos][2]
        if kind in ('SYMBOL', 'STRING'):
            return pos + 1
        if kind == 'LPAREN':
            depth = 1
            j = pos + 1
            while j < n and depth > 0:
                k = tokens[j][2]
                if k == 'LPAREN':
                    depth += 1
                elif k == 'RPAREN':
                    depth -= 1
                j += 1
            return j  # points to token after the matching RPAREN
        # Not a valid S-expression start; skip nothing.
        return pos

    while i < n:
        kind = tokens[i][2]

        # --- NEWLINE --------------------------------------------------------
        if kind == 'NEWLINE':
            i += 1
            continue

        # --- Line comment ---------------------------------------------------
        if kind == 'LINE_COMMENT_BEGIN':
            i += 1  # consume LINE_COMMENT_BEGIN
            if i < n and tokens[i][2] == 'LINE_COMMENT_CONTENT':
                i += 1  # consume LINE_COMMENT_CONTENT
            continue

        # --- Block comment --------------------------------------------------
        if kind == 'BLOCK_COMMENT_BEGIN':
            depth = 1
            i += 1
            while i < n and depth > 0:
                k = tokens[i][2]
                if k == 'BLOCK_COMMENT_BEGIN':
                    depth += 1
                elif k == 'BLOCK_COMMENT_END':
                    depth -= 1
                i += 1
            if depth > 0:
                print(
                    "Error: Unterminated block comment",
                    file=sys.stderr,
                )
            continue

        # --- Unmatched BLOCK_COMMENT_END / BLOCK_COMMENT_TEXT --------------
        if kind in ('BLOCK_COMMENT_END', 'BLOCK_COMMENT_TEXT'):
            print(
                f"Error: Unexpected {kind} at {tokens[i][0]}:{tokens[i][1]}",
                file=sys.stderr,
            )
            i += 1
            continue

        # --- S-expression comment -------------------------------------------
        if kind == 'SEXP_COMMENT_BEGIN':
            i += 1  # consume SEXP_COMMENT_BEGIN
            next_i = skip_sexp(i)
            if next_i == i:
                print(
                    f"Error: SEXP_COMMENT_BEGIN at"
                    f" {tokens[i-1][0]}:{tokens[i-1][1]}"
                    f" not followed by a valid S-expression",
                    file=sys.stderr,
                )
            i = next_i
            continue

        # --- Pass through ---------------------------------------------------
        out.append(tokens[i])
        i += 1

    return out


def main():
    if len(sys.argv) >= 2:
        with open(sys.argv[1], 'r', encoding='utf-8') as f:
            lines = f.readlines()
    else:
        lines = sys.stdin.readlines()

    tokens = read_tokens(lines)
    result = remove_comments(tokens)

    for lnum, col, kind, value in result:
        print(f"{lnum}\t{col}\t{kind}\t{value}")


if __name__ == '__main__':
    main()
