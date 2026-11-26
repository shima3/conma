# Command lines in ConJS

The plan of writing shell commands in a ConJS file: a line that begins with `$ ` runs an external command or a command function, with redirections from and to the variables of the program. The user proposed it on 2026-11-26; M2 wrote this plan the same day, with the questions it raises. M1 reviewed it in shima3/conma#94, and the user decided two of the questions after reading the review. On 2026-11-27 the user decided where the queue and the loop of a command line are: both on the host, with the VM changed so that it can hold values of the host (sections 7 and 9). Section 13 records the review and the decisions.

**State**: a plan. Nothing is built. Section 12 lists what the user is to decide; each question there says what this plan suggests. D3, D12 and D13 are decided.

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

The user added on 2026-11-26, after M1's review (section 13):

* A command function is called with a dollar sign before its name: `$ $sleep 1`. A name without it is always an external command (D3).
* `${...}` holds a variable only, not an expression (D12).

And on 2026-11-27, after the second version (translated):

* The `Queue` is on the host, and the VM is changed so that it can hold values of JavaScript (D13).
* The loop of a command line runs on the host. The ConMa VM is used as little as possible.

## 3. The syntax of a command line

* **Found before the lexer of JavaScript.** As the `async` functions of CONJS-HOST-PLAN.md (section 34) are found before parsing, so are command lines, but on the characters of the file, not on its tokens. The words of a command line are not JavaScript: `$ echo it's` has a lone `'`, and `` $ grep `x` f `` has backquotes; the lexer of JavaScript would start a string or a template there and misread the rest of the file. So a scan over the characters, which knows only where strings, template literals and comments of JavaScript begin and end (ConJS has no regular expressions), finds `$ ` at the start of a line, after spaces or tabs only, outside those. It takes the line, up to its end (with the lines that a `\` at the end of a line joins to it), as raw text. The lexer of JavaScript never sees that text: it sees one token in its place, a command-line token that carries the text and its position. Only the idea is taken from `Conjs_async_at` (shima3/conma#92, conjs/include/conjs_async.se), not the way it works, which looks at tokens.
* **Always a command line.** Such a line is a command line even where `$ .foo` or `$ (x)` would be an expression of JavaScript; the rare program that means the expression writes `$.foo` or `($ (x))`. No `.js` or `.con.js` file of conjs, sjs or consh has a line that begins with `$ ` (M1 checked it), so no program changes its meaning.
* **Only where a statement can begin.** The parser takes the command-line token as a whole statement, and only where a statement can begin: at the top level, in a function body, in a block, as the body of `if`, `for` or `while`. Anywhere else it is an error of the translator that names the line: after `const x =`, in a list of arguments, in an array or an object literal, after a binary operator. A command line does not end an unfinished statement before it, as a semicolon would (that would make `const x =` into `const x = ;`).
* **Words.** Words are separated by spaces. This plan suggests single and double quotes as in a shell (`'a b'` is one word, `"x ${y}"` is one word with the value of `y` in it), a backslash at the end of a line to go on to the next, and `#` to start a comment after a space. No glob, no `~`, no `$HOME` and no `$(...)` at first (D1); a program that wants them writes JavaScript and passes the result with `${...}`.
* **`${name}`** holds the name of a variable, and nothing else (D12, decided): letters, digits, `_` and `$`, not starting with a digit. `${a.b}`, `${f(x)}` and `${ x }` are errors of the translator. The scan of the line needs no lexer of JavaScript for them.
* **What else.** `|`, `&&`, `||`, `;`, `&` and `2>` are left for later stages (section 11). Until then they are an error of the translator, not ordinary words, so that adding them later changes no program that works.

## 4. Arguments

* **No shell runs it.** The words go to the program as its arguments (`execve` on macOS and Linux), never through `/bin/sh`. A value in `${...}` is never split at spaces, nor read for quotes or other syntax. One `${x}` is one argument, whatever characters are in it, so a value cannot inject a command.
* **Turning a value into a string.** A string as it is; a number, a Boolean, `null` and `undefined` as `String(x)` gives them. This plan suggests that an array gives one argument for each element (each turned into a string in the same way), and that `${x}` inside a longer word (`--name=${x}`) takes an array as an error (D2). An object is `String(x)` (`[object Object]`), as in a template literal.
* **The name of the command.** The first word. `${cmd}` as the first word is an external command whose name is the value.

## 5. Which commands are functions

Decided by the user (D3): the first word says it.

* `$ $name ...` calls the command function in the variable `name`. `name` is a variable the line can see, as in a call `name(...)`; when there is none, it is a `ReferenceError` when the line runs, and when it holds no function, a `TypeError`, as for a call.
* Any other first word is an external command, found in `PATH` when it has no `/`. A function of the program, an import or a built-in of ConJS (`sleep`, `spawn`, `join`, `fs`, `http`, `host`, ...) with the same name does not change that: `$ sleep 1` runs the command `sleep` of the OS.
* The translator then needs no rule of scopes for commands, and a name cannot change meaning when a declaration is added elsewhere.

## 6. Redirections

* `<$ name`, `>$ name` and `>>$ name` are written with spaces around them, as the proposal writes them. `name` is a variable that the line can see. At most one input and one output for each command.
* `>$ name` assigns to the variable, so it must be declared (`let out;`) as for any assignment (D4). The line does not declare it, so that a line inside a block does not declare a name that the code after the block cannot see.
* **Without a redirection** (this plan's suggestion; D5):
    * an external command reads the standard input and writes the standard output of the program, as in a shell;
    * a command function gets an iterator of the lines of the standard input of the program, and a queue whose elements are written to the standard output of the program, each turned into a string with a newline after it, as `<$` does for an external command.
* **Into an external command** (`<$`): each element, a string as it is or another value turned into a string as in section 4, then `\n`. The writing is done while the command runs, by the loop on the host (section 9), so an iterator that waits (a generator that reads a file, a queue that another task fills) is fine. When the iterator ends, the standard input of the command is closed.
* **From an external command** (`>$`, `>>$`): lines end at `\n` and `\r\n`, as for `readLines()` in conjs/README.md; a last line with no newline is still a line; empty output gives no element. The bytes are read as UTF-8; bytes that are not UTF-8 become U+FFFD, as `TextDecoder` does.
* **To and from a command function**: the elements are passed as they are: the same objects, so `===` holds and a change made through one reference is seen through the other. `>$` gives a new queue, which is closed when the function returns or throws. `>>$` gives the queue in the variable, which the line does not close (section 7).

## 7. Queues

ConJS has no queue that waits yet. This plan adds one, `Queue`, as a built-in.

* `q.push(x)` adds an element; `q.close()` ends it (any number of times); `q.closed` says whether it is closed. Pushing to a closed queue is an Error.
* It is iterable: `for (const x of q)` takes the elements in order, waits (only the calling task) when the queue is empty and not closed, and ends when it is closed and empty. Each element is taken by one reader: two tasks that read the same queue share its elements, as with a channel of Go.
* `new Queue()` makes one, so that a program can fill a queue itself and pass it with `<$`, or give a queue to `>>$` the first time.
* **Who closes it.** `>$` makes a queue that the line closes when the command ends. `>>$` never closes the queue: the program closes it when every command that writes to it has ended (D6). This keeps one rule for any number of writers.
* **How big it gets.** A command line waits until the command ends (section 8), so the queue of `>$` holds all the output of the command; the reader runs after it. That is unbounded, like `$(...)` in a shell. A queue with a bound, whose `push` waits when it is full, is needed when a reader and a writer run at the same time (a line in a `spawn`, or the pipes of section 11), and is left for that stage (D7).
* **Where it lives: on the host** (D13, decided by the user on 2026-11-27). The `Queue` is an object of the host's JavaScript, and the lines of an external command go into it on the host, with no step of the VM. Today values that cross between the VM and the host are copies (CONJS-HOST-PLAN.md, sections 3.2, 13 and 16), so an object of ConJS put into a queue on the host and taken out again would be another object. Stage S0 (section 11) changes the wasm VM so that values are passed by reference both ways:
    * **Values of the host in the VM.** A new kind of value of the VM names an object of the host by a number. The VM can hold it in variables, arrays and objects, pass it, and compare it with `===` (the same number). Reading a member of it or calling a method of it is done by the host (a call to the host, as `host.wait` is, but one that does not wait unless the method waits: `q.push(x)` does not, taking the next element of an empty queue does). When the VM drops the last reference to it (`Drop` in the Rust of the VM), the host forgets it (removes it from its table).
    * **Values of the VM on the host, kept.** Today a handle of a value of the VM (vm/src/prim/jsfn.rs, `handles`) is good only during one call. A handle that the host keeps in a `Queue` is kept until the host drops it (`FinalizationRegistry`), and the VM keeps the value alive until then. A copy of an object of the VM that a host function made (CONJS-HOST-PLAN.md 3.2) goes into a queue as the object of the VM it is a copy of (by the table of 3.2), so that `===` holds when it is taken out.
    * **What it costs and what it does not do** (M1's answer to the user, section 13): a cycle through the VM and the host (an object of the VM that holds a queue that holds that object) is collected by neither side and lives until the program ends; the native VMs have no host, so they have no `Queue` (D10); each element that the VM puts in or takes out crosses to the host once.

## 8. What a command line gives

* **It waits.** The task that runs a command line waits until the command ends (a call that waits; other tasks run meanwhile). To run commands at the same time, run each in a task with `spawn`.
* **Exit status** (D8). This plan suggests: when an external command ends with a status other than 0, or by a signal, the line throws a `CommandError` (an `Error` with `status`, `signal` and `command`), as zx and `set -e` do. A program that expects other statuses catches it. A command function that throws passes its exception on; what it returns is not used.
* **The output when it fails.** `>$ name` assigns the new queue to the variable before the command starts. When the command fails, the queue holds the lines it wrote before it ended and is closed, and then the line throws. `CommandError` does not carry the output: the program reads it from the variable.
* **The standard error** of an external command goes to that of the program. `2>$` is left for later (section 11).
* **A value** (D9). A command line is a statement and has no value at first. `const r = $ ...` or a `$` in an expression would need a syntax of its own, and is left for later.

## 9. Where it runs

* **The loop runs on the host** (decided by the user on 2026-11-27). The work of a command line is one function of the host's JavaScript, `$sh.run`: it starts the process, feeds its standard input, puts its lines into the queue and waits for its end. The VM only calls it, and does for it only what needs the VM: the next element of an iterator of the VM, and a call of a command function of the VM.
* **How the host waits without stopping the other tasks.** M1 pointed out (section 13) that the host cannot take the elements of `<$ lines` from an iterator of the VM by calling the VM back, since a call back cannot wait for another task of the VM. CONJS-HOST-PLAN.md already has the way round it: a waiting host function is a JavaScript generator that, where it must wait, yields a request to the VProc of its task and is resumed with the result, while the other tasks run (section 20); and a `for...of` over a value of the VM in such a generator takes each element by such a request, which may wait (`$iterG`, `$moreG`, `$takeG`; section 31, which is on `alpha` and reaches `main` with the next merge of `alpha`, as sjs/include/sjs_host.se does). `$sh.run` is a waiting host function of the runtime, written the same way:
    * an element of an iterator of the VM (a generator of ConMa, the lines of `fs.stdin.readLines()`) is taken by a request, which may wait; an array or a string of the host, and a `Queue`, are stepped through on the host, with no request;
    * waiting for the process (its end, or room in the pipe of its standard input) is a request that waits for a Promise of the host, as `host.wait` does;
    * a command function of the VM is called by a request (`$callG`), which may wait; one that is a host function is called on the host.
* **What the translator writes.** A command line becomes one call of `$sh.run` with the parsed line as data and the values it names: `$ grep -n ${pat} <$ lines >$ hits` becomes, in outline, `hits = $sh.run(["grep", "-n", ["var", 0]], { input: 1, output: "new" }, [pat, lines])`. In a waiting host function it is `(yield* $sh.run(...))`; in code that stays in ConMa it is a call of a waiting host function from ConMa, which CONJS-HOST-PLAN.md already does. Either way the VM does one call for the line, and a request for each element of an iterator of the VM.
* **What `$sh.run` does** for an external command, in outline:

    ```
    function* run(words, redir, values) {
      const argv = expand(words, values);              // section 4
      const p = child_process.spawn(argv[0], argv.slice(1), { stdio: [...] });
      const out = redir.output === "new" ? new Queue() : values[redir.output];
      // stdout: Node.js pushes each line into out as it comes (no step of the VM)
      if (redir.input !== undefined) {
        for (const x of yield* iterate(values[redir.input])) {   // a request only for a value of the VM
          if (!p.stdin.write(line(x))) yield* waitFor(drained(p.stdin));
        }
        p.stdin.end();
      }
      const { status, signal } = yield* waitFor(exited(p));
      if (redir.output === "new") out.close();
      if (status !== 0) throw new CommandError(...);  // section 8
      return out;
    }
    ```

    The standard output is read by Node.js into the queue while the loop feeds the standard input, so a command that writes much before it reads all of its input does not stop with both pipes full. The variable of `>$` gets the queue before the command starts (section 8): `$sh.run` hands it to the VM by a request first (in outline above it is only returned).
* **A command function**: `$ $f a ${b} <$ xs >$ out` is `out = new Queue()`, then the call `f(["a", b], xs, out)`, then `out.close()` (also when `f` throws). `out` is a value of the host held by the VM (section 7): when `f` is a function of the VM, its `out.push(x)` goes to the host. The pipes of S3 use the same pieces.
* **On Node.js** (the wasm VM run by vm/wasm/vm-node.js): processes are started with `child_process.spawn`, with no shell.
* **On Windows** (D14): Node.js starts `.exe` and `.com` files found in `PATH`, but without a shell it refuses `.cmd` and `.bat` files (with `EINVAL`, since the fix of 2024), and `npm`, `npx` and many other tools are `.cmd` files. The commands of Git Bash are `.exe`, so the tests of S1 to S3 run on I7, but `$ npm test` does not. Running `.cmd` and `.bat` files needs `cmd.exe`, whose rules of quoting differ, and a value in `${...}` could then inject a command unless it is escaped for `cmd.exe`. This plan suggests: not at first; the line throws an Error that names the file and says to write `$ cmd /c ...` with care.
* **In a browser**: there are no external commands. Command functions work; an external command throws an Error that says so.
* **On the Rust, transpiled and Python VMs**: these have no JavaScript. CONJS-HOST-PLAN.md section 7 asks that a program print the same on every VM, and with the loop and the `Queue` on the host, a command line and a `Queue` need a host. There are two choices (D10): command lines and `Queue` on the wasm VM only (on the other VMs, a command line and `new Queue()` throw an Error that says so), or a second version of `$sh.run` and of `Queue` in ConMa for the native VMs (on `__OS_spawn__` and the channels of include/core/chan.se, as ConSh does), which goes against using the VM as little as possible. This plan suggests the wasm VM only.

## 10. ConSh and this plan

* ConSh is a shell: its scripts are close to Bash, and JavaScript enters only in `js` statements and `(( ))` (SJS). This plan is a JavaScript program: commands enter only on `$ ` lines.
* They share no code at first. ConSh's parser (consh/include/consh_parser.se) reads all of Bash's syntax, far more than section 3; the words of a command line are few enough to be read by the scan of section 3.
* What may be shared later: the Rust helper for job control (section 11), and the rules of ConSh for an external command that is stopped by Ctrl-C (consh/README-ja.md: `$?` 130) and for the processes left when the program ends (consh/test step13).

## 11. Stages

0. **S0, values by reference between the VM and the host** (section 7): values of the host held by the VM, and values of the VM kept by the host, on the wasm VM. Its own tests in conjs/test (`===` of an object put into a host object and taken out, a value of the host dropped by the VM and forgotten by the host, a value of the VM kept by the host and not collected).
1. **S1, external commands.** `$ ` lines with words, quotes and `${...}`, and `<$`, `>$`, `>>$`, on Node.js only, with `$sh.run` on the host (section 9). `Queue` on the host, without a bound. `CommandError`. Ctrl-C stops the running command and the program, as a shell script does.
2. **S2, command functions.** `$ $name`, with the iterator of the standard input and the queue of the standard output for a function without redirections.
3. **S3, pipes.** `a | b | c` with external commands and functions mixed: two external commands are joined by an OS pipe (as consh/test step15 and step8 of ConSh do), a function and a command by a `Queue` with a bound, all on the host. `2>$`, `&&` and `||`.
4. **S4, jobs and the terminal.** `&` (a task from `spawn` that runs the line), and, for an interactive use, job control: the foreground process group of the terminal, Ctrl-Z, `fg` and `bg`. Node.js cannot call `setpgid` or `tcsetpgrp`, so a small helper in Rust starts the commands in a process group of their own and gives it the terminal. Windows has no process groups of this kind: S4 is for macOS and Linux (D11).
5. **Later, if wanted**: a ConMa version for the native VMs (D10), glob and `~` (D1), a command line as an expression (D9), `.cmd` and `.bat` on Windows (D14).

Each stage gets a step of conjs/test, with a `.node.js` version beside it where the output can be compared (conjs/test/compare.sh); on Windows (I7), S1 to S3 are run with Git Bash's commands.

## 12. What the user is to decide

* **D1.** Words: quotes, `\` at the end of a line and `#` comments as section 3 says, and no glob, `~` or `$HOME`? (Suggested: yes.)
* **D2.** `${arr}` with an array: one argument for each element, and an error inside a longer word? (Suggested: yes. The proposal says "turned into a string", which would give `a,b`.)
* **D3.** Decided (2026-11-26): a command function is written `$ $name`; any other name is an external command (section 5).
* **D4.** `>$ name` assigns to a variable that must already be declared? (Suggested: yes.)
* **D5.** Without a redirection, external commands use the program's standard input and output, and functions get its lines and a queue that prints? (Suggested: yes.)
* **D6.** `>>$` never closes the queue; the program closes it? (Suggested: yes.)
* **D7.** A queue without a bound in S1, and one with a bound when readers and writers run at the same time (S3)? (Suggested: yes.)
* **D8.** A status other than 0 throws `CommandError`? Or the line sets a value such as `$?` and goes on? (Suggested: throw.)
* **D9.** A command line is only a statement at first? (Suggested: yes.)
* **D10.** Command lines and `Queue` on the wasm VM only, with an Error on the other VMs? Or a ConMa version for the native VMs too? (Suggested: the wasm VM only, as the VM is to be used as little as possible.)
* **D11.** Job control (S4) for macOS and Linux only? And is it needed at all, or are scripts enough? (Suggested: leave S4 until S1 to S3 are used.)
* **D12.** Decided (2026-11-26): `${...}` holds a variable only (section 3).
* **D13.** Decided (2026-11-27): the `Queue` is on the host, the VM is changed to hold values of the host and to let the host keep values of the VM (section 7, stage S0), and the loop of a command line runs on the host (section 9).
* **D14.** On Windows, `.cmd` and `.bat` files: not run at first, with an Error that says so? (Suggested: yes.)

## 13. The review (shima3/conma#94)

M1 reviewed the first version (2fb40e0) on 2026-11-26. What it found, and what changed:

1. **A `Queue` on the host would not pass the elements as they are.** Values that cross between the VM and the host are copies, so an object put in and taken out would be another object. The second version (631e184) kept the `Queue` in the VM; the third keeps it on the host, with the change of the VM below (section 7).
2. **The host cannot take the elements of `<$` from the VM.** It would call the VM back for each, and a call back cannot wait for another task. M1 suggested that the translator write the loop in ConJS, with small calls that wait on the host, and the second version did that. The third version, after the user's decision below, runs the loop on the host as a waiting host function, which takes each element of an iterator of the VM by a request that may wait, as a `for...of` over a value of the VM does already (CONJS-HOST-PLAN.md sections 20 and 31; section 9).
3. **A command line must be cut out on the characters, not on the tokens**, because its words are not JavaScript (`it's`, backquotes). And a `$ ` line where a statement cannot begin is an error, not the end of the statement before it. Section 3 now says both.
4. **The built-in names of ConJS** (`sleep`, `spawn`, ...) would make `$ sleep 1` call ConJS's `sleep`. Solved by the user's decision D3.
5. **To be added to the questions**: what `${...}` may hold (decided by the user: a variable only, D12), and `.cmd` and `.bat` files on Windows (D14).
6. **Small things**: section 8 now says what the variable of `>$` holds when the command fails.

The user asked, after reading the review, whether the VM could be changed so that the host can keep values of the VM as they are. M1 answered that the wasm VM could: a handle of a value of the VM (vm/src/prim/jsfn.rs, `handles`), now good only during one call, would be kept until the host drops it (`FinalizationRegistry`), and a value of the VM would be added for an object of the host, removed from the host's table when the VM drops it (`Drop`). Then a `Queue` on the host could hold handles, and `===` would hold. M1 noted that a cycle through the VM and the host would be collected by neither, that the native VMs have no host, and that each element would still cross to the host and back. It would not solve point 2.

After the second version, the user decided (2026-11-27) that the `Queue` is on the host, with the VM changed to hold values of JavaScript, and that the loop runs on the host, the ConMa VM being used as little as possible. The third version does that: section 7 (the change of the VM, stage S0), section 9 (`$sh.run`, a waiting host function), D10 (the wasm VM only) and D13 (decided).
