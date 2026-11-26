# Command lines in ConJS

The plan of writing shell commands in a ConJS file: a line that begins with `$ ` runs an external command or a command function, with redirections from and to the variables of the program. The user proposed it on 2026-11-26; M2 wrote this plan the same day, with the questions it raises.

**State**: a plan. Nothing is built. Section 12 lists what the user is to decide; each question there says what this plan suggests.

## 1. Why

* ConJS already writes a call that waits as an ordinary call (conjs/README.md): `f.readLine()` and `sleep(ms)` need no `await`. Running a command and waiting for it to end fits that form: a command line is a statement, and only the task that runs it waits. Libraries of Node.js that do the same, such as zx, need `await` before each command.
* The proposal treats an external command and a function of the program in the same way: the input is an iterator and the output is a queue. Lines of a command and values of the program then flow through the same variables, and a function can stand where a command stood.
* ConSh (consh/) is a shell written in ConMa, close to Bash. This plan is the other way round: a JavaScript program with command lines in it (section 10).

## 2. The proposal

As the user wrote it (translated):

* A line of a `.con.js` file that begins with `$ ` (a dollar sign and a space) is a command line.
* A command is an external command or a command function.
    * A command function takes three arguments: the array of the arguments of the command, the iterator it reads from, and the queue it writes to.
    * The queue it writes to is also iterable.
* For an external command:
    * `${name}` in an argument passes the value of the JavaScript variable `name`, turned into a string.
    * `<$ name` passes the iterator in the variable to the standard input. A string element is written as it is, any other element is turned into a string, and each gets a newline after it.
    * `>$ name` puts a new queue in the variable, and the standard output goes into it.
    * `>>$ name` sends the standard output into the queue already in the variable.
        * The queue gets one string for each line.
* For a command function:
    * `<$ name` passes the iterator in the variable as the input.
    * `>$ name` puts a new queue in the variable and passes it as the output.
    * `>>$ name` passes the queue already in the variable as the output.
        * The elements of the iterator and of the queue are passed as they are, not turned into strings.
* As much as possible is made by the translation of ConJS into JavaScript; the ConMa VM is used as little as possible.
* What is hard in Node.js alone, such as job control, goes through a helper written in Rust.

## 3. The syntax of a command line

* **Where it is found.** As for the `async` functions of CONJS-HOST-PLAN.md (section 34), the lexer finds a command line before parsing (`Conjs_async_at` of shima3/conma#92, in conjs/include/conjs_async.se, is the model): `$ ` at the start of a line, after spaces or tabs only, outside a string, a template literal and a comment. Such a line is always a command line, even where `$ .foo` or `$ (x)` would be an expression of JavaScript; the rare program that means the expression writes `$.foo` or `($ (x))`. A command line is a whole statement: it ends the statement before it, as a semicolon would.
* **Where it may stand.** Anywhere a statement may: at the top level, in a function, in a loop or in a block. `spawn(() => {` on one line, the command line on the next, and `})` after it, runs it as a task.
* **Words.** Words are separated by spaces. This plan suggests single and double quotes as in a shell (`'a b'` is one word, `"x ${y}"` is one word with the value of `y` in it), a backslash at the end of a line to go on to the next, and `#` to start a comment after a space. No glob, no `~`, no `$HOME` and no `$(...)` at first (section 12, D1); a program that wants them writes JavaScript and passes the result with `${...}`.
* **What else.** `|`, `&&`, `||`, `;`, `&` and `2>` are left for later stages (section 11). Until then they are an error of the translator, not ordinary words, so that adding them later changes no program that works.

## 4. Arguments

* **No shell runs it.** The words go to the program as its arguments (`execve` on macOS and Linux), never through `/bin/sh`. A value in `${...}` is never split at spaces, nor read for quotes or other syntax. One `${x}` is one argument, whatever characters are in it, so a value cannot inject a command.
* **Turning a value into a string.** A string as it is; a number, a Boolean, `null` and `undefined` as `String(x)` gives them. This plan suggests that an array gives one argument for each element (each turned into a string in the same way), and that `${x}` inside a longer word (`--name=${x}`) takes an array as an error (section 12, D2). An object is `String(x)` (`[object Object]`), as in a template literal.
* **The name of the command.** The first word. A name in `${...}` is always an external command.

## 5. Which commands are functions

* The translator decides it, from the first word: when the name is a variable or a function declared in a scope that the line can see (the translator already knows the names in scope; shima3/conma#92 uses them to find the free names of an async function), it is a command function. Otherwise it is an external command, found in `PATH` when it has no `/`.
* When the variable holds no function when the line runs, it is a `TypeError`, as a call of it would be.
* A name can be forced to be external with a path (`/bin/ls`, `./run.sh`) or with a word of its own (section 12, D3; for example `$ command ls`).

## 6. Redirections

* `<$ name`, `>$ name` and `>>$ name` are written with spaces around them, as the proposal writes them. `name` is a variable that the line can see. At most one input and one output for each command.
* `>$ name` assigns to the variable, so it must be declared (`let out;`) as for any assignment (section 12, D4). The line does not declare it, so that a line inside a block does not declare a name that the code after the block cannot see.
* **Without a redirection** (this plan's suggestion; section 12, D5):
    * an external command reads the standard input and writes the standard output of the program, as in a shell;
    * a command function gets an iterator of the lines of the standard input of the program, and a queue whose elements are written to the standard output of the program, each turned into a string with a newline after it, as `<$` does for an external command.
* **Into an external command** (`<$`): each element, a string as it is or another value turned into a string as in section 4, then `\n`. The writing is done while the command runs, by the task that runs the line, so an iterator that waits (a generator that reads a file, a queue that another task fills) is fine. When the iterator ends, the standard input of the command is closed.
* **From an external command** (`>$`, `>>$`): lines end at `\n` and `\r\n`, as for `readLines()` in conjs/README.md; a last line with no newline is still a line; empty output gives no element. The bytes are read as UTF-8; bytes that are not UTF-8 become U+FFFD, as `TextDecoder` does.
* **To and from a command function**: the elements are passed as they are. `>$` gives a new queue, which is closed when the function returns or throws. `>>$` gives the queue in the variable, which the line does not close (section 7).

## 7. Queues

ConJS has no queue that waits yet. This plan adds one, `Queue`, as a built-in.

* `q.push(x)` adds an element; `q.close()` ends it (any number of times); `q.closed` says whether it is closed. Pushing to a closed queue is an Error.
* It is iterable: `for (const x of q)` takes the elements in order, waits (only the calling task) when the queue is empty and not closed, and ends when it is closed and empty. Each element is taken by one reader: two tasks that read the same queue share its elements, as with a channel of Go.
* `new Queue()` makes one, so that a program can fill a queue itself and pass it with `<$`, or give a queue to `>>$` the first time.
* **Who closes it.** `>$` makes a queue that the line closes when the command ends. `>>$` never closes the queue: the program closes it when every command that writes to it has ended (section 12, D6). This keeps one rule for any number of writers.
* **How big it gets.** A command line waits until the command ends (section 8), so the queue of `>$` holds all the output of the command; the reader runs after it. That is unbounded, like `$(...)` in a shell. A queue with a bound, whose `push` waits when it is full, is needed when a reader and a writer run at the same time (a line in a `spawn`, or the pipes of section 11), and is left for that stage (section 12, D7).
* **Where it lives.** A queue is a value of the host's JavaScript (section 9). In the VM, include/core/chan.se has channels written in ConMa (and CHAN-PLAN-ja.md is the plan of channels as primitives); the Queue of a native VM (section 9) is built on them.

## 8. What a command line gives

* **It waits.** The task that runs a command line waits until the command ends (a call that waits; other tasks run meanwhile). To run commands at the same time, run each in a task with `spawn`.
* **Exit status** (section 12, D8). This plan suggests: when an external command ends with a status other than 0, or by a signal, the line throws a `CommandError` (an `Error` with `status`, `signal` and `command`), as zx and `set -e` do. A program that expects other statuses catches it. A command function that throws passes its exception on; what it returns is not used.
* **The standard error** of an external command goes to that of the program. `2>$` is left for later (section 11).
* **A value** (section 12, D9). A command line is a statement and has no value at first. `const r = $ ...` or a `$` in an expression would need a syntax of its own, and is left for later.

## 9. Where it runs

* **The translation.** The translator turns a command line into a call of a function of the runtime with the parsed line as data: the words, which of them are `${...}` (as closures or values of variables), the command function if any, and the redirections. For example `$ grep -n ${pat} <$ lines >$ hits` becomes, in outline, `hits = __conjs_shell.run(["grep", "-n", pat], { input: lines, output: "new" })`. The work of a command line is in that one function, not in ConMa code.
* **On Node.js** (the wasm VM run by vm/wasm/vm-node.js): the function is host JavaScript, as the async functions of CONJS-HOST-PLAN.md section 34 are. An external command is started with `child_process.spawn` (no shell), and the line is a call that waits through `host.wait` until the command ends, so other tasks of ConJS go on running. Writing `<$` and reading `>$` also run on the host.
* **In a browser**: there are no external commands. Command functions work; an external command throws an Error that says so.
* **On the Rust, transpiled and Python VMs**: these have no JavaScript. CONJS-HOST-PLAN.md section 7 asks that a program print the same on every VM, so there are two choices (section 12, D10): run command lines on Node.js only (other VMs throw an Error that says so), or write the same function in ConMa on `__OS_spawn__` and the channels, as ConSh does. This plan suggests Node.js only at first, and the ConMa version later if the user wants it, since the proposal asks for the VM as little as possible.

## 10. ConSh and this plan

* ConSh is a shell: its scripts are close to Bash, and JavaScript enters only in `js` statements and `(( ))` (SJS). This plan is a JavaScript program: commands enter only on `$ ` lines.
* They share no code at first. ConSh's parser (consh/include/consh_parser.se) reads all of Bash's syntax, far more than section 3; the words of a command line are few enough to be read in the lexer of ConJS.
* What may be shared later: the Rust helper for job control (section 11), and the rules of ConSh for an external command that is stopped by Ctrl-C (consh/README-ja.md: `$?` 130) and for the processes left when the program ends (consh/test step13).

## 11. Stages

1. **S1, external commands.** `$ ` lines with words, quotes and `${...}`, and `<$`, `>$`, `>>$`, on Node.js only. `Queue` without a bound. `CommandError`. Ctrl-C stops the running command and the program, as a shell script does.
2. **S2, command functions.** The scope rule of section 5, the iterator of the standard input and the queue of the standard output for a function without redirections.
3. **S3, pipes.** `a | b | c` with external commands and functions mixed: two external commands are joined by an OS pipe (as consh/test step15 and step8 of ConSh do), a function and a command by a queue with a bound. `2>$`, `&&` and `||`.
4. **S4, jobs and the terminal.** `&` (a task from `spawn` that runs the line), and, for an interactive use, job control: the foreground process group of the terminal, Ctrl-Z, `fg` and `bg`. Node.js cannot call `setpgid` or `tcsetpgrp`, so a small helper in Rust starts the commands in a process group of their own and gives it the terminal. Windows has no process groups of this kind: S4 is for macOS and Linux (section 12, D11).
5. **Later, if wanted**: the ConMa version for the native VMs (D10), glob and `~` (D1), a command line as an expression (D9).

Each stage gets a step of conjs/test, with a `.node.js` version beside it where the output can be compared (conjs/test/compare.sh); on Windows (I7), S1 to S3 are run with Git Bash's commands.

## 12. What the user is to decide

* **D1.** Words: quotes, `\` at the end of a line and `#` comments as section 3 says, and no glob, `~` or `$HOME`? (Suggested: yes.)
* **D2.** `${arr}` with an array: one argument for each element, and an error inside a longer word? (Suggested: yes. The proposal says "turned into a string", which would give `a,b`.)
* **D3.** Forcing an external command when a function of the same name is in scope: a path only, or also a word such as `command`? (Suggested: both.)
* **D4.** `>$ name` assigns to a variable that must already be declared? (Suggested: yes.)
* **D5.** Without a redirection, external commands use the program's standard input and output, and functions get its lines and a queue that prints? (Suggested: yes.)
* **D6.** `>>$` never closes the queue; the program closes it? (Suggested: yes.)
* **D7.** A queue without a bound in S1, and one with a bound when readers and writers run at the same time (S3)? (Suggested: yes.)
* **D8.** A status other than 0 throws `CommandError`? Or the line sets a value such as `$?` and goes on? (Suggested: throw.)
* **D9.** A command line is only a statement at first? (Suggested: yes.)
* **D10.** Command lines on Node.js only at first, with an Error on the other VMs? Or the same on every VM from the start? (Suggested: Node.js first.)
* **D11.** Job control (S4) for macOS and Linux only? And is it needed at all, or are scripts enough? (Suggested: leave S4 until S1 to S3 are used.)
