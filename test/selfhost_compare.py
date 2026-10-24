#!/usr/bin/env python3
"""selfhost_compare.py — Compare the output of a Python front end tool with
the output of its ConMa version (see docs/SELFHOST.md).

Usage:
    selfhost_compare.py tokens <python_output> <conma_output>
    selfhost_compare.py ast    <python_output> <conma_output>

tokens: <python_output> is the tab-separated output of bin/lexer or
        bin/comment_remover; <conma_output> holds one
        (Token (line col) KIND "value") per line, as __Token_toSexp__ writes.
ast:    <python_output> is the S-expression output of bin/parser,
        bin/includer or bin/resolver; <conma_output> holds the same
        S-expressions, as __AST_toSexp__ writes them (layout is ignored).

A file name "-" reads standard input.

Exit codes:
    0   The outputs are equal.
    1   They differ; the first difference is printed.
    2   A wrong command line or an output that cannot be read.
"""

import re, sys

class FormatError(Exception): pass

def read_text(path):
    # newline="" keeps a "\r\n" inside a block comment as it is
    if path == "-":
        return sys.stdin.buffer.read().decode("utf-8")
    with open(path, encoding="utf-8", newline="") as f:
        return f.read()

# ── Tokens ────────────────────────────────────────────────────────────────────

# The start of a token line: line TAB col TAB kind TAB
_TSV_START = re.compile(r'(\d+)\t(\d+)\t([A-Z_]+)\t')

# NEWLINE values are written as escape sequences by bin/lexer
_NEWLINE_VALUES = {r'\n': "\n", r'\r\n': "\r\n"}

def read_tsv_tokens(text):
    """Read bin/lexer output into a list of (line, col, kind, value).
    A BLOCK_COMMENT_TEXT value may hold newlines, so a line that does not
    start a token continues the value of the previous one."""
    tokens = []
    lines = text.split("\n")
    if lines and lines[-1] == "": lines.pop()
    for n, line in enumerate(lines, 1):
        m = _TSV_START.match(line)
        if m:
            value = line[m.end():]
            if m.group(3) == "NEWLINE": value = _NEWLINE_VALUES.get(value, value)
            tokens.append([int(m.group(1)), int(m.group(2)), m.group(3), value])
        elif tokens:
            tokens[-1][3] += "\n" + line
        else:
            raise FormatError(f"line {n}: not a token: {line!r}")
    return [tuple(t) for t in tokens]

_ESCAPES = {"n": "\n", "t": "\t", "r": "\r", '"': '"', "\\": "\\"}

def unescape(body):
    out, i = [], 0
    while i < len(body):
        ch = body[i]
        if ch == "\\" and i + 1 < len(body) and body[i + 1] in _ESCAPES:
            out.append(_ESCAPES[body[i + 1]]); i += 2
        else:
            out.append(ch); i += 1
    return "".join(out)

def read_conma_tokens(text):
    """Read __Token_toSexp__ output into a list of (line, col, kind, value)."""
    tokens = []
    for n, item in enumerate(read_sexps(text), 1):
        if not (isinstance(item, list) and len(item) == 4 and item[0] == "Token"
                and isinstance(item[1], list) and len(item[1]) == 2
                and all(isinstance(x, str) and x.isdigit() for x in item[1])
                and isinstance(item[2], str) and not item[2].startswith('"')
                and isinstance(item[3], str) and item[3].startswith('"')):
            raise FormatError(f"item {n}: not a Token: {sexp_to_str(item)}")
        tokens.append((int(item[1][0]), int(item[1][1]), item[2], unescape(item[3][1:-1])))
    return tokens

def compare_tokens(py, cm):
    for i, (a, b) in enumerate(zip(py, cm), 1):
        if a != b:
            return f"token {i} differs:\n  python: {a!r}\n  conma:  {b!r}"
    if len(py) != len(cm):
        i = min(len(py), len(cm))
        more = "python" if len(py) > len(cm) else "conma"
        return (f"python has {len(py)} tokens, conma has {len(cm)}\n"
                f"  {more} token {i + 1}: {(py if len(py) > len(cm) else cm)[i]!r}")
    return None

# ── S-expressions ─────────────────────────────────────────────────────────────

# A String keeps its quotes and escapes, so Strings compare as source text
_SEXP_TOKEN = re.compile(r'\s*(?:(\()|(\))|("(?:[^"\\]|\\.)*")|([^\s()"]+))', re.S)

def read_sexps(text):
    """Read a sequence of S-expressions. A list is a Python list; an atom or a
    String is a str (a String with its double quotes)."""
    stack, top, pos = [], [], 0
    while True:
        m = _SEXP_TOKEN.match(text, pos)
        if not m:
            if text[pos:].strip():
                raise FormatError(f"offset {pos}: cannot read: {text[pos:pos + 20]!r}")
            break
        pos = m.end()
        if m.group(1):
            stack.append([])
        elif m.group(2):
            if not stack: raise FormatError(f"offset {m.start(2)}: unexpected ')'")
            item = stack.pop()
            (stack[-1] if stack else top).append(item)
        else:
            atom = m.group(3) or m.group(4)
            (stack[-1] if stack else top).append(atom)
    if stack: raise FormatError("unexpected end: missing ')'")
    return top

def sexp_to_str(x):
    return "(" + " ".join(sexp_to_str(y) for y in x) + ")" if isinstance(x, list) else x

def _short(x, limit=100):
    s = sexp_to_str(x)
    return s if len(s) <= limit else s[:limit] + " ..."

def _is_pos(x):
    return isinstance(x, list) and x and all(isinstance(y, str) for y in x) and x[0].isdigit()

def _label(x):
    """The head of a node, to show where it is: the kind, the position and the
    atoms before the first child node, such as 'Variable (9 3) "h"' or 'G 0 "f"'."""
    if not isinstance(x, list) or not x: return sexp_to_str(x)
    parts = [x[0]]
    rest = x[1:]
    if rest and _is_pos(rest[0]):
        parts.append(sexp_to_str(rest[0]))
        rest = rest[1:]
    for y in rest:
        if isinstance(y, list): break
        parts.append(y)
    return " ".join(parts)

def compare_sexp(a, b, path):
    """Return a message for the first difference, or None."""
    if isinstance(a, list) and isinstance(b, list):
        here = path + [_label(a)]
        for x, y in zip(a, b):
            msg = compare_sexp(x, y, here)
            if msg: return msg
        if len(a) != len(b):
            i = min(len(a), len(b))
            more = "python" if len(a) > len(b) else "conma"
            return (f"at {' > '.join(here)}: python has {len(a)} items, conma has {len(b)}\n"
                    f"  {more} item {i + 1}: {_short((a if len(a) > len(b) else b)[i])}")
        return None
    if a != b:
        return (f"at {' > '.join(path)}:\n  python: {_short(a)}\n  conma:  {_short(b)}")
    return None

def compare_ast(py, cm):
    for i, (a, b) in enumerate(zip(py, cm), 1):
        msg = compare_sexp(a, b, [f"item {i}"])
        if msg: return msg
    if len(py) != len(cm):
        i = min(len(py), len(cm))
        more = "python" if len(py) > len(cm) else "conma"
        return (f"python has {len(py)} S-expressions, conma has {len(cm)}\n"
                f"  {more} item {i + 1}: {_short((py if len(py) > len(cm) else cm)[i])}")
    return None

# ── Main ──────────────────────────────────────────────────────────────────────

def main(argv):
    if len(argv) != 4 or argv[1] not in ("tokens", "ast"):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    mode, py_path, cm_path = argv[1:]
    py_read, cm_read = ((read_tsv_tokens, read_conma_tokens) if mode == "tokens"
                        else (read_sexps, read_sexps))
    data = []
    for side, path, read in (("python", py_path, py_read), ("conma", cm_path, cm_read)):
        try:
            data.append(read(read_text(path)))
        except (OSError, UnicodeDecodeError, FormatError) as e:
            print(f"error: {side} output {path}: {e}", file=sys.stderr)
            return 2
    msg = (compare_tokens if mode == "tokens" else compare_ast)(*data)
    if msg:
        print(msg)
        return 1
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
