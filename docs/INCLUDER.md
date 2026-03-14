# Specification of `includer`

## Overview

`includer` is a multi-file AST driver for ConMa source files.

For each source file, it runs the pipeline:

```
lexer <file> | comment_remover | parser --file <file>
```

The parser outputs an **S-expression AST** as defined in `AST.md`.

`includer` scans this AST to locate `Includer` nodes, resolves the referenced files, and processes them recursively.

The AST of each processed file is written to **standard output exactly as produced by the pipeline**, in the order the files are processed.

Errors from any stage are written to **standard error**.
On error the program exits with a non-zero status.

---

# Processing Loop

The program maintains a **file list** of absolute, `realpath`-normalised paths and a **file index** initialised to 0. The loop runs as follows:

1. Take the path at position `file_index` in the file list. Call it `FILE`.
2. Run the pipeline `lexer FILE | comment_remover | parser --file FILE` and capture its standard output as the AST text. If any of the three processes exits with a non-zero return code, write an error to standard error and exit with code 1.
3. Scan the AST text for `Includer` nodes (see **Includer Extraction** below) and obtain the list of referenced filenames in the order they appear.
4. For each referenced filename:
   a. Resolve it to an absolute, `realpath`-normalised path (see **Includer File Resolution** below). If resolution fails, write an error to standard error and exit with code 1.
   b. Search the current file list for a path that is equal to this normalised path (string equality after `realpath` normalisation on both sides).
   c. If a match is found, write a warning to standard error and do not add the path to the file list.
   d. If no match is found, append the path to the end of the file list.
5. Write the AST text to standard output.
6. Increment `file_index` by 1.
7. If `file_index` is less than the current length of the file list, go to step 1. Otherwise, exit normally with code 0.

---

# Includer File Resolution

Given a filename string `name` extracted from an `Includer` node and the directory `base_dir` of the file currently being processed:

- If `name` is an absolute path, it is normalised with `os.path.realpath` and used directly.
- If `name` is a relative path, the following directories are searched in order:
  1. `base_dir` (the directory of the file that contains the `Includer`)
  2. Each directory in the include directory list, in order

  The first directory in which `name` exists as a file is used. The result is normalised with `os.path.realpath`.

  If `name` is not found in any of the above directories, the program writes an error to standard error and exits with code 1.

---

# Includer Extraction

The AST produced by `parser` is an **S-expression**.

An `Includer` node has the form:

```
(Includer (line col)
  (String (line col) "filename"))
```

The filename is the string value of the `String` node that is a child of `Includer`.

### Extraction rule

1. Parse the AST text as a sequence of **S-expression tokens**:

   * `"string"`
   * `(`
   * `)`
   * atom

2. When the pattern

```
( Includer ...
```

is encountered, scan the subtree corresponding to that S-expression.

3. Within that subtree, locate the first `String` node:

```
(String (line col) "filename")
```

4. Extract the string literal `"filename"` and remove the quotes.

5. The resulting string is treated as the **included filename**.

6. Filenames are collected in the order the `Includer` nodes appear in the AST.

---

# Output

For each processed file, the AST text produced by the pipeline is written to standard output **without modification**.

The outputs appear in the order of the **file list**:

1. the initial source file
2. directly included files
3. transitively included files

Files are output in the order they are **first discovered**.

No separator is inserted between consecutive AST outputs.

---

# Circular Include Handling

If an included file resolves to a path that already exists in the file list (after `realpath` normalisation), the include is ignored and the program writes a warning to standard error:

```
Warning: circular include ignored: '<path>' (from '<file>')
```

The file is not added again to the file list.
