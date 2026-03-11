# Design of interpreter the ConMa programming language

## Lexer and Comment Remover

### Why comments are preserved by the lexer

The lexer (`lexer2.py`) emits all tokens including comment-related tokens (`LINE_COMMENT_BEGIN`, `LINE_COMMENT_CONTENT`, `BLOCK_COMMENT_BEGIN`, `BLOCK_COMMENT_TEXT`, `BLOCK_COMMENT_END`, `SEXP_COMMENT_BEGIN`) and `NEWLINE`. Comments are not silently discarded during lexical analysis.

This separation ensures that each component has a single, well-defined responsibility:

- The lexer is responsible for segmenting the source text into tokens.
- The comment remover is responsible for deciding which tokens the parser should see.

It also allows the raw token stream (including comments) to be used independently — for example, by documentation generators or syntax highlighters — without modifying the lexer.

### Why NEWLINE tokens are removed by the comment remover, not the lexer

`NEWLINE` tokens serve a purpose within the raw token stream: they mark the boundary between a `LINE_COMMENT_CONTENT` token and the subsequent code, allowing the comment remover to correctly identify where a line comment ends. Once comment removal is complete, `NEWLINE` tokens carry no further meaning for the parser, since ConMa's grammar does not use newlines as statement terminators.

Removing `NEWLINE` in the lexer would conflate two distinct decisions — "what is a token?" and "what does the parser need?" — into a single component. Removing them in the comment remover keeps each stage's scope narrow and explicit.

### Pipeline

```
lexer  →  comment_remover  →  parser
```

## Execution Pipeline

The ConMa frontend consists of a driver script, `includer`, which manages the overall transformation from multiple source files to a single AST stream.

1. **`includer`**: Manages the file list using a Breadth-First Search (BFS) strategy. It resolves `Includer` nodes and avoids infinite loops by normalizing paths with `realpath`.
2. **`lexer`**: Tokenizes raw text, preserving comments and newlines.
3. **`comment_remover`**: Filters out comments and newlines to produce a clean token stream.
4. **`parser`**: Generates the AST. When invoked by `includer`, it uses the `--file` flag to attach the filename to the `Program` node and perform automatic `SourceInfo` insertion.

## AST Metadata

As specified in `AST.md`, location information (`@line:col`) is attached to every node. For multi-file support, the `Program` node is uniquely appended with the quoted filename to serve as a delimiter in the unified output stream.

*Distinction between Meta-Location and SourceInfo Data*: the coordinates in the `@line:col` suffix are numeric, whereas the line and column values embedded inside a `SourceInfo` node value are String literals enclosed in double quotes, in accordance with the grammar `SourceInfo = "(", "__SI__", { String }, ")"`.

## Execution

If the `__CChain_pop_CFrame__` primitive is invoked when the **Continuation Chain (CChain)** is empty, the following behavior is guaranteed:

1. **Null Passing**: The interpreter passes a `null` value as the argument to the current **Local Continuation (LCont)**.
2. **Execution Resumption**: The `LCont` is then executed with this `null` value, allowing the program to handle the end of the chain or an empty state through standard null-check logic (e.g., using `__is_null__`).

**Operational Note:**
This design ensures that a VP (Virtual Process) does not abruptly terminate or enter an undefined state when the chain is exhausted. Instead, it provides a predictable signal (`null`) to the functional layer, maintaining the language's emphasis on explicit control flow.
