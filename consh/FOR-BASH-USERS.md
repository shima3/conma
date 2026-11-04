# ConSh for Bash users

ConSh is a shell that runs on the ConMa language. Its syntax looks like Bash, and simple scripts often read the same, but its values are different: a variable holds either a string or a *list*, and the output of a command is a sequence of *elements* (lines) rather than a stream of bytes. This guide explains how to run ConSh and what changes when you come from Bash.

The full specification is in [PLAN-ja.md](PLAN-ja.md) (Japanese), section 1; section 1.3 lists the differences from Bash. Every example below was run with ConSh; the output is shown after `# ->`.

## Running ConSh

```
bin/consh script.sh [arg ...]     # run a script
bin/consh                         # interactive mode
bin/consh --conma-parser script.sh  # parse with the ConMa parser instead of Python
```

* In interactive mode the prompts are `PS1` and `PS2` (default `$ ` and `> `). An error does not end the shell; it prints a message, sets `$?` to 1 and shows the next prompt. Ctrl-C stops only the running command (`$?` is 130), and Ctrl-D ends the shell.
* If [rlwrap](https://github.com/hanslub42/rlwrap) is installed and standard input is a terminal, `bin/consh` uses it for line editing and history (`~/.consh_history`). Set `CONSH_NO_RLWRAP=1` to turn this off.
* Each simple statement takes thousands of ConMa steps, so loops are slow: a loop of 1000 `echo` commands takes 5 to 10 seconds. External commands connected by pipes run at full speed.

## What stays the same

* Simple commands, `;`, `|`, `&`, comments, and line continuation with `\`.
* Single quotes, double quotes, backslash escapes and `$'...'` (ANSI-C quoting).
* `$x`, `${x}`, `$?`, `$!`, `$$`, `$0`, `$1`…, `$#`, `$@` at the top level of a script.
* `if … then … elif … else … fi` and `for x in …; do … done`, with redirections after `fi` and `done`.
* Redirections `<`, `>`, `>>`, `2>`, `2>&1`, `1>&2`.
* Globs `*`, `?`, `[...]`; a pattern that matches nothing stays as it is.
* External commands, `cd`, `wait`, `kill`, `exit`, `builtin`, `command`.
* `$?` of a pipeline is the status of its last element; a command that cannot be started gives 127, a killed job 143.

## What is missing

ConSh has a small grammar. These Bash features are syntax errors or are not special:

| Bash | In ConSh |
|---|---|
| `while`, `until`, `case`, `select` | not available; use `for`, `if` and functions (see [Idioms](#idioms-for-missing-features)) |
| `a && b`, `a \|\| b`, `! a` | not available; use `if` |
| `{ …; }`, `( … )` | not available; use a function, or a pipeline element or `$( )` for a separate context |
| `f() { … }` | write `function f () { … }` |
| `<<END`, `<<<`, `2>>`, `&>` | not available; see [Redirecting to and from variables](#redirecting-to-and-from-variables) |
| `$(( … ))`, `${#x}`, `${x:-y}`, `${x%…}` and other parameter operators | not available; `${x-y}` is the only form with an operator; use external commands such as `expr` |
| `{a,b}` brace expansion | syntax error (`{` and `}` must be quoted) |
| `` `cmd` ``, `$*`, `~` | not special: they stay as they are |
| `local`, `read`, `set`, `shift`, `source`, `test`/`[` as builtins, aliases, arrays | not builtins; `[` and `test` work as the external commands |

The builtin commands are `echo`, `cd`, `defined`, `builtin`, `command`, `stdin`, `wait`, `kill` and `exit`. `echo` takes no options.

## Variables

### No word splitting

Unquoted variables are never split on spaces. A value with spaces stays one word:

```sh
x="a b  c"
/bin/echo $x          # -> a b  c      (one argument, spaces kept)
```

Quoting `"$x"` is therefore needed less often than in Bash. To get several words, use a list.

### Lists

A variable can hold a list, written `(word ...)`:

```sh
list=(one two three)
for w in $list; do echo "item $w"; done
# -> item one
# -> item two
# -> item three
```

Lists are lazy: their elements are made when they are read, so a list can be infinite (see `$(yes)` below). A list may contain lists.

How a list is expanded depends on how you write it:

| Form | To an external command | To a function or builtin | On the right of `y=` |
|---|---|---|---|
| `$list` | one word per leaf | one word per leaf | the one leaf; an error if there are 2 or more |
| `${list}`, `$(cmd)` | one word per leaf | **the list as one value** | the list |
| `"$list"` | one string | one string | one string |

A *leaf* is a string found by walking into nested lists. So `y=$list` with two elements is an error (`2 words in a variable definition`); write `y=${list}` to copy a list.

When a list becomes one string (`"$list"`, `export`, or writing it as one line), its leaves are joined with the variable `OFS`, a space if `OFS` is not defined:

```sh
echo "$list"            # -> one two three
OFS=, echo "$list"      # -> one,two,three
```

Text around a list is added to every leaf, and two lists give every combination: if `list` is `(a b)`, `x$list.txt` gives `xa.txt xb.txt`, and `$list$list` gives `aa ab ba bb`.

### Undefined variables are errors

Referring to a variable that is not defined stops the script, like Bash's `set -u`:

```sh
echo $nosuch
# -> consh: run6.sh:1:6: undefined variable: nosuch
```

Use `${x-default}` for a default, and the builtin `defined` to test:

```sh
echo ${undefined-fallback}       # -> fallback
defined undefined; echo "$?"     # -> 1
```

The default in `${x-…}` is read as a word even inside double quotes, so `"${x-a b}"` is a syntax error; write `"${x-"a b"}"`.

An empty string disappears when unquoted: with `x=""`, `$x` gives no word and `"$x"` gives an empty word, as in Bash.

### Local variables and the environment

ConSh keeps shell variables and environment variables apart:

* At start-up, environment variables are not copied into shell variables, but `$HOME` and others can be read while no shell variable has the same name.
* A shell variable wins over an environment variable of the same name.
* `export X=v` sets only the environment; `X=v` sets only the shell variable; `export X` copies the shell variable to the environment.

```sh
HOGE=a; export HOGE=b; echo $HOGE    # -> a
```

A prefix assignment such as `x=1 cmd` sets the environment of `cmd` and, unlike Bash, is also seen by the words of the same statement when there is no shell variable `x`:

```sh
x=1 echo $x      # -> 1   (Bash prints the old value)
```

## Functions

Functions are defined with `function`, and their parameters are named in a list. `. name` collects the rest of the arguments as a list:

```sh
function greet (name . rest) {
  echo "hello $name"
  echo "rest: $rest"
  return 3
}
greet Alice Bob Carol; echo "status $?"
# -> hello Alice
# -> rest: Bob Carol
# -> status 3
```

* Missing arguments are empty strings (or an empty list for the rest parameter); extra arguments are dropped.
* Inside a function, `$1`, `$@` and `$#` are errors. Use named parameters; `function f (. args)` takes all of them.
* `return` needs a number from 0 to 255: a bare `return` is a syntax error.
* Variables set in a function are local to it. Functions see variables by lexical scope, that is, the values at the point where the function was defined:

```sh
x=1
function show () { echo "x is $x" }
x=2
show                  # -> x is 1   (Bash prints 2)
```

* A function can call functions defined later, and itself.
* A function with the name of a builtin replaces it; `builtin echo` and `command /bin/echo` reach the original.

### Passing arguments and lists

To pass the script arguments on to a function as separate arguments, use `$@`. `"$@"` is **one string** in ConSh:

```sh
# consh script.sh "a b" c
function count (. args) { for a in ${args}; do echo "arg: $a"; done }
count $@
# -> arg: a b
# -> arg: c
echo "[$@]"       # -> [a b c]
```

To pass a list to a function as one value, use `${list}`; `$list` passes its leaves as separate arguments:

```sh
list=(p q r)
function first (x) { echo "first: $x" }
first ${list}     # -> first: p q r   (x is the whole list)
first $list       # -> first: p
```

## Output is a sequence of elements

Each `echo` writes one element, and the lines of an external command become elements when they go to a function, a builtin, `$( )` or a variable list. At the terminal or in a file each element is one line.

### Command substitution

`$(cmd)` is a list of the output lines, not a string. Lines are not split on spaces:

```sh
/usr/bin/printf 'one two\nthree four\n' > lines.txt
for line in $(/bin/cat lines.txt); do echo "<$line>"; done
# -> <one two>
# -> <three four>
```

`"$(cmd)"` joins the lines with `OFS`, a space by default; Bash keeps the newlines. Set `OFS=$'\n'` to keep them:

```sh
/bin/echo "$(/usr/bin/printf 'a\nb\n')"                 # -> a b
OFS=$'\n'; /bin/echo "$(/usr/bin/printf 'a\nb\n')"      # -> a, then b on the next line
```

`$( )` runs alongside the code that reads it, so an endless command works as long as you stop reading:

```sh
for x in $(/usr/bin/yes); do echo "got $x"; break; done    # -> got y
```

Variable definitions, `export`, `cd` and `return` inside `$( )` do not affect the outside, as in a Bash subshell.

### Redirecting to and from variables

`> (a b . rest)` puts the first output element in `a`, the second in `b`, and the remaining ones, as a list, in `rest`. This replaces `read` and many uses of `$( )`:

```sh
function numbers () { echo 1; echo 2; echo 3 }
numbers > (a b . rest)
echo "a=$a b=$b rest=$rest"        # -> a=1 b=2 rest=3
/bin/date +%Y > (year)             # year is the first line
/bin/ls /nonexistent 2> (err)      # err is the first line of standard error
```

The command is stopped once all the variables are filled, with `$?` 0. As in Bash, variables set by the last element of a pipeline (`echo "a b" | tr … > (a b)`) are lost when the pipeline ends; put the redirection on a statement of its own. `< (a b . rest)` does the reverse and replaces here-documents:

```sh
lines=(alpha beta gamma)
/usr/bin/sort -r < (. lines)
# -> gamma
# -> beta
# -> alpha
```

### Reading standard input

The builtin `stdin` copies its standard input to its standard output, so `stdin > (line)` reads one line, like `read line`. A function and its caller share the read position:

```sh
function upper () {
  stdin > (first . rest)
  echo "first=$first"
  for line in ${rest}; do echo "more=$line"; done
}
/usr/bin/printf 'a\nb\nc\n' | upper
# -> first=a
# -> more=b
# -> more=c
```

### Pipes

Every element of a pipeline runs in its own lightweight process, so, as in Bash, variables set in a pipeline element are lost.

* Between two functions or builtins, elements are passed one by one, lists included. This is slow (about 1000 ConMa steps per element).
* If either side is an external command, the two are joined by an OS pipe and bytes flow as they are.
* `$?` is the status of the last element; there is no `pipefail` or `PIPESTATUS`.
* When the reader ends first, the writer stops the next time it writes, so `yes | head -1` ends.

Output that passes through elements may differ from the original bytes: a final line without a newline gets one, and `\r\n` becomes `\n`.

## Control flow

`if` works as in Bash. The `then` or `else` part may be empty, which replaces `!`:

```sh
if /usr/bin/grep -q zzz lines.txt; then else echo "not found"; fi
```

`for` is the only loop, and it only iterates over words. `for x in ${list}` and `for x in $(cmd)` read the list lazily, so they work on long or endless output.

`break` and `continue` take an optional **loop variable name** instead of a count:

```sh
for i in 1 2 3; do
  for j in a b c; do
    if [ $j = b ]; then continue i; fi
    echo $i$j
  done
done
# -> 1a
# -> 2a
# -> 3a
```

You cannot `break` out of a loop from inside a function, a pipeline element or `$( )`.

## Globs

Globs work as in Bash (`.` files need a pattern that starts with `.`). ConSh adds `${-pattern}`, which gives the matching names as a list and an **empty list** when nothing matches, so a loop over it does not run:

```sh
echo /etc/host*              # -> /etc/hosts /etc/hosts.equiv …
echo /nonexistent/*          # -> /nonexistent/*
echo ${-/nonexistent/*}      # -> (an empty line)
for f in ${-*.txt}; do echo $f; done    # runs once per .txt file, or not at all
```

Assignments do not expand globs (`x=*.txt` stores `*.txt`), but an unquoted `$x` does later, as in Bash.

## Jobs

`cmd &` runs in the background; `$!` is its job number (a ConSh process, not an OS process ID). `wait` with a job number gives its status; `wait` alone waits for all jobs. At the end of a script, ConSh waits for jobs that are still running.

```sh
/bin/sleep 1 &
wait $!; echo "status $?"                 # -> status 0
/bin/sleep 10 &
kill $!; wait $!; echo "status $?"        # -> status 143
```

There is no `fg`, `bg` or `jobs`. A background job shares standard input with the shell (Bash gives it `/dev/null`).

## Idioms for missing features

| Bash | ConSh |
|---|---|
| `a && b` | `if a; then b; fi` |
| `a \|\| b` | `if a; then else b; fi` |
| `while cond; do …; done` | `for _ in $(/usr/bin/yes); do if cond; then else break; fi; …; done`, or a recursive function |
| `for ((i=0; i<3; i++))` | `for i in $(/usr/bin/seq 0 2)` |
| `n=$((n + 1))` | `n=$(/bin/expr $n + 1)` |
| `read line` | `stdin > (line)` |
| `read a b <<< "$line"` (split one line) | `/usr/bin/tr " " "\n" < (line) > (a b)` |
| `mapfile -t lines < file` | `lines=$(/bin/cat file)` |
| `cat <<END … END` | `text=(line1 line2); cmd < (. text)` |
| `"${array[@]}"` | `$list` (to external commands) or `${list}` (to functions) |
| `cmd 2>/dev/null` | the same |
| `cmd &> file` | `cmd > file 2>&1` |
| `x=$(cd dir; cmd)` | the same: `cd` inside `$( )` does not change the shell's directory |
| `local x=1` | `x=1` inside a function (always local) |
| `"$@"` | `$@` |
| `$*`, `"$*"` | `"$@"` |
| `${x:-def}` | `${x-def}` (an empty `x` counts as defined) |

A counting loop written with `expr`:

```sh
i=0
for _ in $(/usr/bin/yes); do
  if [ $i -ge 3 ]; then break; fi
  echo "tick $i"
  i=$(/bin/expr $i + 1)
done
# -> tick 0
# -> tick 1
# -> tick 2
```

## Errors and exit status

* An error that the specification names (an undefined variable, `$1` in a function, `return 300`, a syntax error, …) prints `consh: file:line:column: message` and ends the script with status 1. In interactive mode it only ends the current command.
* A command that cannot be started prints a message and sets `$?` to 127; a file that cannot be opened and a failing `cd` set it to 1. The script continues.
* `exit n` in a pipeline element or `$( )` ends only that element: `echo a | exit 2` sets `$?` to 2.
* A script ends with the `$?` of its last statement.
