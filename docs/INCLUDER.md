## Specification of `includer`

### Overview

`includer` is a multi-file AST driver for ConMa source files. It resolves `Includer` nodes recursively, runs the processing pipeline on each source file in order, and writes the resulting `Program` nodes to standard output as a continuous stream per `AST.md` Section 5.

The pipeline run for each file is:

```
lexer <file>  |  comment_remover  |  parser --file <file>
```

Errors from any stage are written to standard error. On error, the program exits with a non-zero exit code.

---

### Invocation

```
includer [--bin <dir>] [--include <dir>] ... <source_file>
```

#### Positional argument

- `<source_file>`: path to the top-level ConMa source file. May be absolute or relative. If relative, it is resolved against the current working directory.

#### Options

- `--bin <dir>`: directory in which to search for `lexer`, `comment_remover`, and `parser`. If omitted, the current working directory is used. Specified as a single directory; may not be repeated.
- `--include <dir>`: additional directory to search when resolving relative paths in `Includer` nodes. May be specified multiple times. Directories are searched in the order they appear on the command line.

---

### Startup

#### Tool resolution

At startup, the program locates `lexer`, `comment_remover`, and `parser` as files inside the `--bin` directory (or the current working directory if `--bin` is not given). If any of the three tools is not found, the program writes an error to standard error and exits with code 1.

The `--bin` path itself is normalised with `os.path.realpath` before use. If the resulting path is not an existing directory, the program writes an error to standard error and exits with code 1.

#### Include directory resolution

Each `--include` directory is normalised with `os.path.realpath`. If the resulting path is not an existing directory, the program writes an error to standard error and exits with code 1. The normalised paths are stored in an ordered list (the **include directory list**) in the order they were specified.

#### Initial source file resolution

The `<source_file>` argument is resolved to an absolute path as follows:

- If it is already absolute, it is normalised with `os.path.realpath`.
- If it is relative, it is joined to the current working directory and then normalised with `os.path.realpath`.

If the resulting path does not refer to an existing file, the program writes an error to standard error and exits with code 1.

The resolved absolute path is added as the sole initial entry of the **file list**.

---

### Processing Loop

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

### Includer File Resolution

Given a filename string `name` extracted from an `Includer` node and the directory `base_dir` of the file currently being processed:

- If `name` is an absolute path, it is normalised with `os.path.realpath` and used directly.
- If `name` is a relative path, the following directories are searched in order:
  1. `base_dir` (the directory of the file that contains the `Includer`)
  2. Each directory in the include directory list, in order

  The first directory in which `name` exists as a file is used. The result is normalised with `os.path.realpath`.

  If `name` is not found in any of the above directories, the program writes an error to standard error and exits with code 1.

---

### Includer Extraction

The AST text is scanned line by line to find `Includer` nodes. The extraction rule is:

1. A line whose content, after stripping leading whitespace, begins with `Includer@` is an `Includer` line.
2. The line immediately following an `Includer` line is examined. If its content, after stripping leading whitespace, begins with `String@` and contains the substring `": "`, the text after `": "` is taken as the filename value.
3. If that text is enclosed in double quotes (`"..."`) the quotes are stripped to obtain the filename string.
4. Filenames are collected in the order the `Includer` lines appear in the AST text.

---

### Output

The `Program` nodes are written to standard output in the order the corresponding files appear in the file list (i.e., the initial source file first, followed by directly and transitively included files in the order they were first encountered). The AST text for each file is written exactly as produced by the pipeline, with no separator between consecutive `Program` nodes.

---

### Error Messages

| Condition | Message written to stderr | Exit code |
|---|---|---|
| `--bin` directory not found | `Error: --bin directory not found: '<dir>'` | 1 |
| Tool not found in `--bin` | `Error: '<tool>' not found in '<dir>'` | 1 |
| `--include` directory not found | `Error: --include directory not found: '<dir>'` | 1 |
| Initial source file not found | `Error: source file not found: '<name>'` | 1 |
| Included file not found | `Error: included file not found: '<name>' (from '<file>')` | 1 |
| Pipeline tool exits non-zero | `Error: <tool> exited with code <n>` | 1 |
| Circular include detected | `Warning: circular include ignored: '<file>' (from '<file>')` | 0 (continues) |
