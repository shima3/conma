# Command lines in ConJS

The plan of writing shell commands in a ConJS file: a line that begins with `$ ` runs an external command or a command function, with redirections from and to the variables of the program. The user proposed it on 2026-11-26; M2 wrote this plan the same day, with the questions it raises. M1 reviewed it in shima3/conma#94, and the user decided two of the questions after reading the review. On 2026-11-27 the user decided where the queue and the loop of a command line are: both on the host, with the VM changed so that it can hold values of the host (sections 7 and 9). The same day the user decided the form of the queue: a queue can have a parent, and `>>$` makes a new queue whose parent is the old one (section 7). Section 13 records the review and the decisions.

**State**: a plan. Stage S0 is built: S0a, values by reference between the VM and the host, below the language (section 14), and S0b, their use from ConJS (section 15). Stages S1 to S3 are built: the `Queue` (S1a, section 16), the command lines with command functions (S1b, section 17; S2 came with it), and the pipelines with `2>$`, `&&` and `||` (S3, section 18). S4 is left until S1 to S3 are used (D11). Section 12 lists the questions and what the user decided. All of them were decided by 2026-11-27 (D1 to D15; the user's last answers are in section 12).

## 1. Why

* ConJS already writes a call that waits as an ordinary call (conjs/README.md): `f.readLine()` and `sleep(ms)` need no `await`. Running a command and waiting for it to end fits that form: a command line is a statement, and only the task that runs it waits. Libraries of Node.js that do the same, such as zx, need `await` before each command.
* The proposal treats an external command and a function of the program in the same way: the input is an iterable and the output is a queue. Lines of a command and values of the program then flow through the same variables, and a function can stand where a command stood.
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

And on 2026-11-27, the form of the queue (translated; relayed by M1 in shima3/conma#94):

* A command function takes three arguments: the array of the arguments of the command, the iterable it reads from, and the queue it writes to.
* The queue it writes to is also iterable.
* A queue is open at first and values can be added to it; once it is closed, none can be added.
* Both `>$` and `>>$` close the queue at the end.
* A new queue can be made with a parent.
* The iterator of a queue gives the values of the parent, then the values of the child.
* `>>$` replaces the queue in the variable with a new queue whose parent is that queue, and passes the new queue as the output.
* An element that is read is gone (the user's answer to M1): readers share the elements, so of two queues with the same parent, an element of the parent read through one is not seen through the other.

## 3. The syntax of a command line

* **Found before the lexer of JavaScript.** As the `async` functions of CONJS-HOST-PLAN.md (section 34) are found before parsing, so are command lines, but on the characters of the file, not on its tokens. The words of a command line are not JavaScript: `$ echo it's` has a lone `'`, and `` $ grep `x` f `` has backquotes; the lexer of JavaScript would start a string or a template there and misread the rest of the file. So a scan over the characters, which knows only where strings, template literals and comments of JavaScript begin and end (ConJS has no regular expressions), finds `$ ` at the start of a line, after spaces or tabs only, outside those. It takes the line, up to its end (with the lines that a `\` at the end of a line joins to it), as raw text. The lexer of JavaScript never sees that text: it sees one token in its place, a command-line token that carries the text and its position. Only the idea is taken from `Conjs_async_at` (shima3/conma#92, conjs/include/conjs_async.se), not the way it works, which looks at tokens.
* **Always a command line.** Such a line is a command line even where `$ .foo` or `$ (x)` would be an expression of JavaScript; the rare program that means the expression writes `$.foo` or `($ (x))`. No `.js` or `.con.js` file of conjs, sjs or consh has a line that begins with `$ ` (M1 checked it), so no program changes its meaning.
* **Only where a statement can begin.** The parser takes the command-line token as a whole statement, and only where a statement can begin: at the top level, in a function body, in a block, as the body of `if`, `for` or `while`. Anywhere else it is an error of the translator that names the line: after `const x =`, in a list of arguments, in an array or an object literal, after a binary operator. A command line does not end an unfinished statement before it, as a semicolon would (that would make `const x =` into `const x = ;`).
* **Words.** Words are separated by spaces. Single and double quotes work as in a shell (D1) (`'a b'` is one word, `"x ${y}"` is one word with the value of `y` in it), a backslash at the end of a line to go on to the next, and `#` to start a comment after a space. No glob, no `~`, no `$HOME` and no `$(...)` at first; a program that wants them writes JavaScript and passes the result with `${...}`.
* **`${name}`** holds the name of a variable, and nothing else (D12, decided): letters, digits, `_` and `$`, not starting with a digit. `${a.b}`, `${f(x)}` and `${ x }` are errors of the translator. The scan of the line needs no lexer of JavaScript for them.
* **What else.** `|`, `&&`, `||`, `;`, `&` and `2>` are left for later stages (section 11). Until then they are an error of the translator, not ordinary words, so that adding them later changes no program that works.

## 4. Arguments

* **No shell runs it.** The words go to the program as its arguments (`execve` on macOS and Linux), never through `/bin/sh`. A value in `${...}` is never split at spaces, nor read for quotes or other syntax. One `${x}` is one argument, whatever characters are in it, so a value cannot inject a command.
* **Turning a value into a string.** A string as it is; a number, a Boolean, `null` and `undefined` as `String(x)` gives them. Any other value is turned into a string the same way, as in a template literal (D2, decided by the user: the proposal's "turned into a string"): an array gives one argument, `String(["a", "b"])`, that is `a,b`, and an object gives `[object Object]`. A program that wants the elements of an array as arguments writes a variable for each.
* **The name of the command.** The first word. `${cmd}` as the first word is an external command whose name is the value.

## 5. Which commands are functions

Decided by the user (D3): the first word says it.

* `$ $name ...` calls the command function in the variable `name`. `name` is a variable the line can see, as in a call `name(...)`; when there is none, it is a `ReferenceError` when the line runs, and when it holds no function, a `TypeError`, as for a call.
* Any other first word is an external command, found in `PATH` when it has no `/`. A function of the program, an import or a built-in of ConJS (`sleep`, `spawn`, `join`, `fs`, `http`, `host`, ...) with the same name does not change that: `$ sleep 1` runs the command `sleep` of the OS.
* The translator then needs no rule of scopes for commands, and a name cannot change meaning when a declaration is added elsewhere.

## 6. Redirections

* `<$ name`, `>$ name` and `>>$ name` are written with spaces around them, as the proposal writes them. `name` is a variable that the line can see. At most one input and one output for each command.
* `>$ name` assigns to the variable, so it must be declared (`let out;`) as for any assignment (D4). The line does not declare it, so that a line inside a block does not declare a name that the code after the block cannot see.
* **Without a redirection** (D5):
    * an external command reads the standard input and writes the standard output of the program, as in a shell;
    * a command function gets an iterable of the lines of the standard input of the program, and a queue whose elements are written to the standard output of the program, each turned into a string with a newline after it, as `<$` does for an external command.
* **Into an external command** (`<$`): each element, a string as it is or another value turned into a string as in section 4, then `\n`. The writing is done while the command runs, by the loop on the host (section 9), so an iterable that waits (a generator that reads a file, a queue that another task fills) is fine. When it ends, the standard input of the command is closed.
* **From an external command** (`>$`, `>>$`): lines end at `\n` and `\r\n`, as for `readLines()` in conjs/README.md; a last line with no newline is still a line; empty output gives no element. The bytes are read as UTF-8; bytes that are not UTF-8 become U+FFFD, as `TextDecoder` does.
* **To and from a command function**: the input is the iterable in the variable of `<$`, as it is (an array, a generator, a `Queue`). The elements are passed as they are: the same objects, so `===` holds and a change made through one reference is seen through the other. The output is the new queue of `>$` or of `>>$` (section 7), closed when the function returns or throws.

## 7. Queues

ConJS has no queue that waits yet. This plan adds one, `Queue`, as a built-in.

The form of the queue was decided by the user on 2026-11-27 (section 2).

* `new Queue()` makes an open queue with no parent; `new Queue(parent)` makes one whose parent is the queue `parent`. A program makes queues itself to fill them and pass them with `<$`, or to give one to `>>$` the first time.
* `q.push(x)` adds an element to `q` itself (not to its parent); `q.close()` closes it (any number of times); `q.closed` says whether it is closed. Pushing to a closed queue is an Error.
* **Reading.** A queue is iterable. `for (const x of q)` first takes the elements of the parent of `q`, as an iterator of the parent would (so the parent's parent comes first of all), until the parent is closed and has no element left; then the elements of `q` itself, until `q` is closed and has none left. It waits (only the calling task) whenever the queue it is taking from is empty and not closed. So a reader of `q` waits until every queue before it is closed, even when `q` itself already has elements.
* **An element read is gone** (the user's answer to M1). Each element is taken by one reader, as with a channel of Go: two tasks that read the same queue share its elements, and of two queues with the same parent, an element of the parent read through one is not seen through the other.
* **`>$ q`** sets `q` to `new Queue()` before the command starts. **`>>$ q`** sets `q` to `new Queue(q)`: the queue that was in `q` becomes the parent, so a reader of `q` gets the output of the earlier lines first, then that of this line, in the order the lines began. The first `>>$ q` needs a queue in `q`, made by `>$` or by the program.
* **Who closes it** (D6, decided). The line closes its new queue when the command ends, for `>$` and for `>>$` alike, also when the command fails or the command function throws. A queue the program made with `new Queue()` is closed by the program; until it is, a reader of a queue that has it as an ancestor waits.
* **`>>$` cannot be cut in two** (M1's point, B). Reading `q`, making `new Queue(q)` and assigning it to `q` must be done with no other task running in between. If two tasks did `>>$ q` at once and both read the same `q` before either assigned it, the output of the one that assigned first would be lost from `q`. In ConJS other tasks run only at a call that waits (conjs/README.md), so it is enough that making a queue is a call to the host that does not wait: not a request of a waiting host function (section 9) and not `host.wait`, both of which let other tasks run. Then the queue of the line that began later has the queue of the earlier line as its parent, and nothing is lost.
* **How big it gets.** A command line waits until the command ends (section 8), so the queue of `>$` holds all the output of the command; the reader runs after it. That is unbounded, like `$(...)` in a shell. A queue with a bound, whose `push` waits when it is full, is needed when a reader and a writer run at the same time (a line in a `spawn`, or the pipes of section 11), and is left for that stage (D7).
* **Where it lives: on the host** (D13, decided by the user on 2026-11-27). The `Queue` is an object of the host's JavaScript, and the lines of an external command go into it on the host, with no step of the VM. Today values that cross between the VM and the host are copies (CONJS-HOST-PLAN.md, sections 3.2, 13 and 16), so an object of ConJS put into a queue on the host and taken out again would be another object. Stage S0 (section 11) changes the wasm VM so that values are passed by reference both ways:
    * **Values of the host in the VM.** A new kind of value of the VM names an object of the host by a number. The VM can hold it in variables, arrays and objects, pass it, and compare it with `===` (the same number). Reading a member of it or calling a method of it is done by the host (a call to the host, as `host.wait` is, but one that does not wait unless the method waits: `q.push(x)` does not, taking the next element of an empty queue does). When the VM drops the last reference to it (`Drop` in the Rust of the VM), the host forgets it (removes it from its table).
    * **Values of the VM on the host, kept.** Today a handle of a value of the VM (vm/src/prim/jsfn.rs, `handles`) is good only during one call. A handle that the host keeps in a `Queue` is kept until the host drops it (`FinalizationRegistry`), and the VM keeps the value alive until then. A copy of an object of the VM that a host function made (CONJS-HOST-PLAN.md 3.2) goes into a queue as the object of the VM it is a copy of (by the table of 3.2), so that `===` holds when it is taken out.
    * **What it costs and what it does not do** (M1's answer to the user, section 13): a cycle through the VM and the host (an object of the VM that holds a queue that holds that object) is collected by neither side and lives until the program ends; the native VMs have no host, so they have no `Queue` (D10); each element that the VM puts in or takes out crosses to the host once.

## 8. What a command line gives

* **It waits.** The task that runs a command line waits until the command ends (a call that waits; other tasks run meanwhile). To run commands at the same time, run each in a task with `spawn`.
* **Exit status** (D8). When an external command ends with a status other than 0, or by a signal, the line throws a `CommandError` (an `Error` with `status`, `signal` and `command`), as zx and `set -e` do. A program that expects other statuses catches it. A command function that throws passes its exception on; what it returns is not used.
* **The output when it fails.** `>$ name` assigns the new queue to the variable before the command starts. When the command fails, the queue holds the lines it wrote before it ended and is closed, and then the line throws. `CommandError` does not carry the output: the program reads it from the variable.
* **The standard error** of an external command goes to that of the program. `2>$` is left for later (section 11).
* **A value** (D9). A command line is a statement and has no value at first. `const r = $ ...` or a `$` in an expression would need a syntax of its own, and is left for later.

## 9. Where it runs

* **The loop runs on the host** (decided by the user on 2026-11-27). The work of a command line is one function of the host's JavaScript, `$sh.run`: it starts the process, feeds its standard input, puts its lines into the queue and waits for its end. The VM only calls it, and does for it only what needs the VM: the next element of an iterator of the VM, and a call of a command function of the VM.
* **How the host waits without stopping the other tasks.** M1 pointed out (section 13) that the host cannot take the elements of `<$ lines` from an iterator of the VM by calling the VM back, since a call back cannot wait for another task of the VM. CONJS-HOST-PLAN.md already has the way round it: a waiting host function is a JavaScript generator that, where it must wait, yields a request to the VProc of its task and is resumed with the result, while the other tasks run (section 20); and a `for...of` over a value of the VM in such a generator takes each element by such a request, which may wait (`$iterG`, `$moreG`, `$takeG`; section 31, which is on `alpha` and reaches `main` with the next merge of `alpha`, as sjs/include/sjs_host.se does). `$sh.run` is a waiting host function of the runtime, written the same way:
    * an element of an iterable of the VM (a generator of ConMa, the lines of `fs.stdin.readLines()`) is taken by a request, which may wait; an array or a string of the host is stepped through on the host, with no request, and so is a `Queue` (taking from an empty one waits for a Promise of the host, as below);
    * waiting for the process (its end, or room in the pipe of its standard input) is a request that waits for a Promise of the host, as `host.wait` does;
    * a command function of the VM is called by a request (`$callG`), which may wait; one that is a host function is called on the host.
* **What the translator writes.** A command line becomes one call of `$sh.run` with the parsed line as data and the values it names: `$ grep -n ${pat} <$ lines >$ hits` becomes, in outline, `hits = new Queue(); $sh.run(["grep", "-n", ["var", 0]], [pat, lines, hits]);`, and with `>>$ hits` the first statement is `hits = new Queue(hits);` (section 7: these do not wait, so no other task runs between the read and the assignment). In a waiting host function the call is `(yield* $sh.run(...))`; in code that stays in ConMa it is a call of a waiting host function from ConMa, which CONJS-HOST-PLAN.md already does. Either way the VM makes the queue, does one call for the line, and a request for each element of an iterable of the VM.
* **What `$sh.run` does** for an external command, in outline:

    ```
    function* run(words, values) {
      const [input, output] = redirections(words, values);
      try {
        const argv = expand(words, values);              // section 4
        const p = child_process.spawn(argv[0], argv.slice(1), { stdio: [...] });
        // stdout: Node.js pushes each line into output as it comes (no step of the VM)
        if (input !== undefined) {
          for (const x of yield* iterate(input)) {       // a request only for a value of the VM
            if (!p.stdin.write(line(x))) yield* waitFor(drained(p.stdin));
          }
          p.stdin.end();
        }
        const { status, signal } = yield* waitFor(exited(p));
        if (status !== 0) throw new CommandError(...);   // section 8
      } finally {
        if (output !== undefined) output.close();        // section 7: >$ and >>$ alike
      }
    }
    ```

    The standard output is read by Node.js into the queue while the loop feeds the standard input, so a command that writes much before it reads all of its input does not stop with both pipes full. The variable of `>$` or `>>$` has its queue before the command starts (section 8), since the VM made it.
* **A command function**: `$ $f a ${b} <$ xs >$ out` is `out = new Queue()` (or `out = new Queue(out)` for `>>$`), then the call `f(["a", String(b)], xs, out)`, then `out.close()` (also when `f` throws). `out` is a value of the host held by the VM (section 7): when `f` is a function of the VM, its `out.push(x)` goes to the host. The pipes of S3 use the same pieces.
* **On Node.js** (the wasm VM run by vm/wasm/vm-node.js): processes are started with `child_process.spawn`, with no shell.
* **On Windows** (D14): Node.js starts `.exe` and `.com` files found in `PATH`, but without a shell it refuses `.cmd` and `.bat` files (with `EINVAL`, since the fix of 2024), and `npm`, `npx` and many other tools are `.cmd` files. The commands of Git Bash are `.exe`, so the tests of S1 to S3 run on I7, but `$ npm test` does not. Running `.cmd` and `.bat` files needs `cmd.exe`, whose rules of quoting differ, and a value in `${...}` could then inject a command unless it is escaped for `cmd.exe`. Not at first (D14): the line throws an Error that names the file and says to write `$ cmd /c ...` with care.
* **Where `$sh.run` and `Queue` are** (M1's review of 9c9e4b3). They are functions of the runtime, not of the program, and the same for every program and every module of it: one `Queue` class, so that a queue made in one module is a `Queue` in another. So they are in `rt` of vm/wasm/conma.js (as `rt.defineHost` and `rt.asyncScope` are; section 34 of CONJS-HOST-PLAN.md), not written by the translator into the text of each host module, as `$iterG` and the other helpers of sjs/include/sjs_host.se are. The host modules of the program reach them through `rt`. ConMa code reaches them by two new ways: a call of `$sh.run` given to `Conjs_hostfn_call` (conjs/include/conjs_host.se), which today runs only the functions of the program's host module (`Conjs_hostfn_module`), so it needs a way to name a function of `rt`; and `new Queue(...)` and the methods of a queue, plain calls of the host that do not wait, except taking from an empty queue (section 7).
* **Which functions wait** (M1's review of 9c9e4b3). A function with a command line in it is a waiting function, as one that calls `sleep` is: it goes into the set that `Hx_wait_loop` of sjs/include/sjs_host.se makes (`"#waits"`), so it is written as a generator and its callers wait too. A function expression cannot be a generator (CONJS-HOST-PLAN.md section 20), so a command line in one, such as the arrow function of `spawn(() => { ... })`, is run by the ConMa version of that function, which calls `$sh.run` through `Conjs_hostfn_call`. That way is not only for code that the translator leaves in ConMa: every command line in a `spawn` takes it. Both ways run the same `$sh.run`.
* **When there is no host** (D15; M1's review of 9c9e4b3). Other host functions fall back to their ConMa versions and print the same; `$sh.run` and `Queue` have no ConMa version (D10). So a program with a command line, or that names the built-in `Queue`, stops with an Error where it cannot have them: on the wasm VM where JavaScript cannot be made from strings (Node.js with `--disallow-code-generation-from-strings`, a page whose Content-Security-Policy forbids `eval`) or where the host is turned off, and on the Rust, transpiled and Python VMs. It stops at its start, before any line runs (D15), as a program with an async function does (conjs/README.md), since the translator knows which programs have them: `conjs: command lines and Queue need the JavaScript of the host (FILE:LINE)`, with the place of the first one, exit status 2. (An Error thrown when the line runs would have let a program run half way and stop.) A program that declares or imports a `Queue` of its own (`class Queue { ... }`) uses that one, as a program that declares `sleep` uses its own `sleep` (conjs/README.md), and does not stop (M1's review of 47e6962).
* **In a browser**: there are no external commands. Command functions work; an external command throws an Error that says so.
* **On the Rust, transpiled and Python VMs**: these have no JavaScript. CONJS-HOST-PLAN.md section 7 asks that a program print the same on every VM, and with the loop and the `Queue` on the host, a command line and a `Queue` need a host. Command lines and `Queue` are on the wasm VM only (D10): on the other VMs the program stops at its start, as when there is no host (D15). A second version of `$sh.run` and of `Queue` in ConMa for the native VMs (on `__OS_spawn__` and the channels of include/core/chan.se, as ConSh does) would go against using the VM as little as possible.

## 10. ConSh and this plan

* ConSh is a shell: its scripts are close to Bash, and JavaScript enters only in `js` statements and `(( ))` (SJS). This plan is a JavaScript program: commands enter only on `$ ` lines.
* They share no code at first. ConSh's parser (consh/include/consh_parser.se) reads all of Bash's syntax, far more than section 3; the words of a command line are few enough to be read by the scan of section 3.
* What may be shared later: the Rust helper for job control (section 11), and the rules of ConSh for an external command that is stopped by Ctrl-C (consh/README-ja.md: `$?` 130) and for the processes left when the program ends (consh/test step13).

## 11. Stages

0. **S0, values by reference between the VM and the host** (section 7): values of the host held by the VM, and values of the VM kept by the host, on the wasm VM. A handle that lives only during one call, as `lines` of the example of section 9 does (a value of the VM passed to `$sh.run`), works today; S0 is for what is kept across calls: a `Queue` held by the VM, and the elements of a queue held by the host (M1's review of 9c9e4b3). Its own tests in conjs/test (`===` of an object put into a host object and taken out, a value of the host dropped by the VM and forgotten by the host, a value of the VM kept by the host and not collected).
1. **S1, external commands.** `$ ` lines with words, quotes and `${...}`, and `<$`, `>$`, `>>$`, on Node.js only, with `$sh.run` on the host (section 9). `Queue` on the host, without a bound. `CommandError`. Ctrl-C stops the running command and the program, as a shell script does.
2. **S2, command functions.** `$ $name`, with the iterable of the lines of the standard input and the queue of the standard output for a function without redirections.
3. **S3, pipes.** `a | b | c` with external commands and functions mixed: two external commands are joined by an OS pipe (as consh/test step15 and step8 of ConSh do), a function and a command by a `Queue` with a bound, all on the host. `2>$`, `&&` and `||`.
4. **S4, jobs and the terminal.** `&` (a task from `spawn` that runs the line), and, for an interactive use, job control: the foreground process group of the terminal, Ctrl-Z, `fg` and `bg`. Node.js cannot call `setpgid` or `tcsetpgrp`, so a small helper in Rust starts the commands in a process group of their own and gives it the terminal. Windows has no process groups of this kind: S4 is for macOS and Linux (D11).
5. **Later, if wanted**: a ConMa version for the native VMs (D10), glob and `~` (D1), a command line as an expression (D9), `.cmd` and `.bat` on Windows (D14).

Each stage gets a step of conjs/test, with a `.node.js` version beside it where the output can be compared (conjs/test/compare.sh); on Windows (I7), S1 to S3 are run with Git Bash's commands.

## 12. The questions, and what the user decided

* **D1.** Words: quotes, `\` at the end of a line and `#` comments as section 3 says, and no glob, `~` or `$HOME`? Decided (2026-11-27): yes.
* **D2.** `${arr}` with an array: one argument for each element, or turned into a string as the proposal says? Decided (2026-11-27): turned into a string, `a,b` (section 4).
* **D3.** Decided (2026-11-26): a command function is written `$ $name`; any other name is an external command (section 5).
* **D4.** `>$ name` assigns to a variable that must already be declared? Decided (2026-11-27): yes.
* **D5.** Without a redirection, external commands use the program's standard input and output, and functions get its lines and a queue that prints? Decided (2026-11-27): yes.
* **D6.** Decided (2026-11-27): the line closes its queue at the end, for `>$` and `>>$` alike; `>>$` makes a new queue whose parent is the old one (section 7).
* **D7.** A queue without a bound in S1, and one with a bound when readers and writers run at the same time (S3)? Decided (2026-11-27): yes.
* **D8.** A status other than 0 throws `CommandError`, or the line sets a value such as `$?` and goes on? Decided (2026-11-27): it throws.
* **D9.** A command line is only a statement at first? Decided (2026-11-27): yes.
* **D10.** Command lines and `Queue` on the wasm VM only, or a ConMa version for the native VMs too? Decided (2026-11-27): the wasm VM only.
* **D11.** Job control (S4): for macOS and Linux only, and is it needed at all? Decided (2026-11-27): leave S4 until S1 to S3 are used.
* **D12.** Decided (2026-11-26): `${...}` holds a variable only (section 3).
* **D13.** Decided (2026-11-27): the `Queue` is on the host, the VM is changed to hold values of the host and to let the host keep values of the VM (section 7, stage S0), and the loop of a command line runs on the host (section 9).
* **D14.** On Windows, `.cmd` and `.bat` files not run at first, with an Error that says so? Decided (2026-11-27): yes.
* **D15.** Without a host, a program with a command line or `Queue` stops at its start, or throws when the line runs? Decided (2026-11-27): at its start, as for async functions (section 9).

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

Then the user decided the form of the queue (2026-11-27), relayed by M1 with two points of M1's: that `>>$` must replace the queue with no other task running in between (B), and that the input of a command function is now an iterable, not an iterator (C). The fourth version takes them in: section 2 (the user's words), section 6 (the input and the output of a command function), section 7 (the parent, reading, closing and `>>$`), section 9 (the translator makes the queue; `$sh.run` closes it) and D6 (decided).

M1 then pointed to three things of its review of 9c9e4b3 that the fourth version had not taken in: where `$sh.run` and `Queue` are (in `rt`), that a function with a command line is a waiting function and that a command line in a function expression goes through the ConMa version, and what happens without a host (D15). The fifth version adds them to section 9, with the scope of S0 (section 11). M1 also checked B: tasks of ConJS change only at a call that waits, so making the queue with a call that does not wait keeps `>>$` whole.

On 2026-11-27 the user answered the rest of the questions (section 12). All followed what this plan suggested but D2: a value of `${...}` that is an array is turned into a string, as the proposal said, not split into arguments (section 4).

## 14. Results of S0a: values by reference (2026-11-27)

S0 is built in two parts: S0a, the values by reference between the VM and the host, which the primitives of the platform feature `"js"` carry (`__JS_fn_call__` and `__JS_fn_resume__`; docs/PRIMITIVES.md), and S0b, the use of them from ConJS code (reading the members of an object of the host and calling its methods, `for...of` over it), which the `Queue` of S1 needs.

* **Objects of the host in the VM.** A new kind of value of the VM, `Value::Host` (vm/src/value.rs), holds the number of an object in a table of the host; `__type_name__` gives `"HostObject"`. An object whose prototype or itself is marked with `rt.byRef` (vm/wasm/conma.js) crosses as `{"o":k}` instead of being copied, and comes back as the same object. The VM keeps one `HostObj` for each number (a table of weak references in vm/src/prim/jsfn.rs), so the same object of the host is the same value of the VM; when the VM drops the last reference, `Drop` calls `host_obj_free`, and the host removes the object from its table.
* **Values of the VM kept by the host.** `rt.keep(v)`, during a call, sends `v` to the VM by a call back (`{"keep":value}`); the VM puts the value in a table (`KEPT`) and answers its number, and the host gets an `rt.VmKept`, which crosses as `{"k":i}` and is the same value of the VM in any later call. A `FinalizationRegistry` calls `conma_jsfn_release` when the host drops the `VmKept`, and the VM lets the value go. The collector of cycles of the VM (vm/src/gc.rs) needs no roots: a value in `KEPT` has a reference from outside the containers, so it stays alive.
* **The native VMs** have no host: the new kind of value never appears there, and `host_obj_free` does nothing.
* **The test**: test/fast22. case1: a box made on the host (a class marked with `rt.byRef`) has the type `"HostObject"`, keeps its state across calls, is still an instance of its class and `===` to itself, and not to another box; a box that the VM drops leaves the table of the host; a box inside a plain object comes inside the copied Dict. case2: a Dict kept by the host comes back as the same Dict (a change made through what came back is seen in the original), and a kept Closure can be called after the call that kept it. case3: when the host drops what it kept, the VM lets the values go: test.sh gives `--expose-gc` to Node.js, the host calls `gc()`, and after the event loop has turned (a `time.sleep` of the host; a `FinalizationRegistry` never calls back in the middle of synchronous code), the table of the VM is empty (M1's review of shima3/conma#95).
* **Not done**: a cycle through the VM and the host is collected by neither (section 7).

## 15. Results of S0b: objects of the host in ConJS (2026-11-27)

* **The mark.** The mark of S0a is the registered symbol `Symbol.for("conma.byRef")` (vm/wasm/conma.js), so that code of the host that has no `rt`, a file of `CONMA_HOST` or an async function of a ConJS file, can mark a class: `Box.prototype[Symbol.for("conma.byRef")] = true`. `host.wait` and `host.call` carry such objects too: `{"o":k}` in their JSON both ways (vm/src/prim/host.rs, `HostValue::Obj`; `hostEncode` and `hostDecode` of conma.js), and so do the arguments and results of async functions, which go the same way. ConJS passes them as they are (`Conjs_to_host` of conjs/include/conjs_rt.se; the first version refused them, M1's review of shima3/conma#95), also inside arrays and objects.
* **The runtime** (sjs/include/sjs_rt.se, "ホストのオブジェクト"): a `HostObject` is of the kind `"host"`, and `typeof` gives `"object"`. Reading a member, writing it, calling a method, opening `for...of`, taking the next value, closing the loop early, `===`, `String(o)` and the name of its class are done on the host by a small module of JavaScript, made once with `__JS_fn_compile__` (its number in the PDict, `"sjs_host_obj"`) and called with `__JS_fn_call__`. These calls do not wait, so no other task runs during them; this keeps `>>$` whole (section 7). What ConJS passes is kept with `rt.keep` unless it is a number, a string, a Boolean or `undefined`, so a value of ConJS put into an object of the host comes back as the same value. What comes back is marked: an object of the host or a kept value as it is, a plain object or an array copied into the values of SJS (as `host.wait` does), an exception of the host as an `Error` of ConJS.
* **Waiting.** A method, or a `next()` of an iterator, that returns a Promise answers with a number; the task waits, as for `host.wait`, on the host function `conma.hostobj.await`, which the module defines, and then takes the answer by its number. Only ConJS can wait: it puts the way to wait in the PDict (`"sjs_host_await"`, conjs/include/conjs_rt.se); in SJS such a method is a TypeError.
* **`===`** asks the host (`same`). The same object of the host is always the same `HostObj` in the VM (section 14), so comparing in the VM would do, but ConMa has no primitive that compares two values for identity; adding one is left for when it is measured to matter (M1's review of shima3/conma#95).
* **Shown**: `console.log` writes `[host Box]`, with the name of the class, not its members (which would each be a call to the host).
* **The test**: conjs/test/step23 (docs/dev/CONJS.md lists what it checks). Functions of ConJS given a `Box` (case2) print the same; whether they run on the host or in ConMa (section 2 of CONJS-HOST-PLAN.md) is not checked.
* **Not done**: a plain object that the host returns and that holds a kept value of ConJS is copied with that value inside it, and the copy reads the value as a plain object too (the kept object of ConJS is a Dict with its own marks); methods return such values rarely, and the `Queue` of S1 returns its elements one by one.

## 16. Results of S1a: the Queue (2026-11-27)

* **On the host** (vm/wasm/conma.js, `rt.Queue`): an array of elements, whether it is closed, the parent and the readers that wait. `take()` asks the parent first, recursively; once the parent answers that it is closed and empty, the queue forgets it (it stays so). Then its own first element, or the end when it is closed, or the queue itself as the one to wait for. `next()` of its iterator returns `{ done, value }`, or a Promise when the reader must wait, which `push` or `close` of the queue that it waits for settles; the reader then tries again. The prototype carries the mark of S0, so a queue goes to the VM by reference.
* **In ConJS**: `Queue` is a built-in (conjs/include/conjs_rt.se). `new Queue(parent)` goes to `"hostnew"` of the function (conjs/include/conjs_class.se, `Conjs_new`), which asks the module of S0b (`queue`) to make one. The rest is what S0b gives to any object of the host: `push`, `close` and `closed` on the host, `for...of` with `next()` waiting by a Promise, the elements kept as they are.
* **The test**: conjs/test/step24 (docs/dev/CONJS.md).
* **Not done**: a bound (D7; S3), `instanceof Queue`, and stopping at the start without a host (D15; with the command lines of S1b).

## 17. Results of S1b: command lines (2026-11-27)

* **A rewriting of the text, not of the tokens** (conjs/include/conjs_shell.se, `Conjs_shell_rewrite`, called by `Sjs_parse_file` before the lexer). The scan of section 3 goes over the lines, keeping whether it is in a block comment and the stack of template literals (with the depth of `{` in each `${ }`); strings end at the end of their line. Only a file with `$ ` anywhere is scanned, so other programs pay nothing. A command line, with the lines that a `\` at its end joins to it, becomes one line of ConJS at the same place, and the joined lines become empty lines, so the positions of every other line stay. The ConJS of the line is a block, `{ let conjs_command_line; ... }`, which a statement can be and an expression cannot: where a statement cannot begin, the parser reads `{` as an object literal and stops at `conjs_command_line` (section 3). Since the line is ConJS, the variables of `${...}`, `<$` and `>$` are found by the ordinary scopes of the translator, and a missing one is a `ReferenceError` at the line, as section 5 asked; the translator itself is not changed.
* **What a line becomes.** An external command: `conjs_command_line = conjs_sh_run([words], input, output, "text")`, then `throw` of what it returns, if anything, so that a `CommandError` or an error of starting has the position of the line. A word is a string literal, `String(name)`, or `[parts].join("")`. `>$ v` writes `v = new Queue();` first and `>>$ v` writes `v = new Queue(v);` first; neither waits, so no task comes between reading and assigning `v` (section 7). A command function: `conjs_command_line = conjs_sh_begin(input, output)`, then `f([words], conjs_command_line.input, conjs_command_line.output)` in a `try` whose `finally` closes the output (and, without `>$`, waits for the task that prints it). The functions `conjs_sh_run`, `conjs_sh_begin` and `conjs_sh_end` are ConJS, added at the end of a file that has command lines (declarations, so they are hoisted, and the lines before them keep their positions). The names that begin with `conjs_sh_` and `conjs_command_line` are taken; ConJS does not allow `__` or `$` in declared names, which the first version used.
* **The loop on the host** (vm/wasm/host/shell.js, the host function `conma.sh.run`, which vm/wasm/vm-node.js adds; section 9). `conjs_sh_run` calls it with `host.wait`, so only the calling task waits, and the queues go by reference (S0). It starts the command with `child_process.spawn` (no shell), feeds its standard input from an array or a `Queue` (waiting for room in the pipe, and for a `Queue` that is empty), reads its standard output into the output queue line by line (UTF-8, `\n` and `\r\n`), closes the queue at the end, and resolves to `{ status, signal }`. An input that is neither an array nor an object of the host (a generator of ConMa) is moved into a `Queue` by a task of ConJS first: taking the elements of an iterable of the VM needs the VM, as section 9 says, and a task is the way ConJS already has to wait for them (the requests of a waiting host function would need `$sh.run` to be a host function made by the translator).
* **The review** (M1, shima3/conma#96), and what changed: waiting for room in the pipe left a listener of `close` each time (a warning of Node.js after 11), and now takes off the one that did not fire; the feeding of an input `Queue` took one more element after the command had stopped reading, and now waits for an element without taking it (`ready()` of `rt.Queue`); the task that moves a generator into a `Queue` never waited, so an endless generator kept every other task from running (`$ head -3 <$ g` never ended), and now waits while 64 elements are in the queue (`room()` of `rt.Queue`) and stops when the line closes the queue after the command. The scan now follows a string that goes on to the next line with `\` (as the lexer of ConJS does), and a command line goes on only when it ends in an odd number of `\`. The functions added at the end of a file use the built-ins by name, so a program that declares one of them at its top level changes them; conjs/README.md says so.
* **D15** (conjs/include/conjs_host.se, `Conjs_shell_check`): `Conjs_shell_rewrite` notes the first command line, or the first `new Queue` in the text (a mention in a comment or a string counts too), in the PDict (`"conjs_shell_at"`), and `Gen_conjs_program` makes the program try `__JS_fn_compile__` first; where it fails, the program stops with `conjs: command lines and Queue need the JavaScript of the host (FILE:LINE): ...` and exit status 1, as for async functions.
* **The test**: conjs/test/step25 (docs/dev/CONJS.md), and `step25 (no eval)` of conjs/test/run_all.sh.
* **Not done**: S2 to S4 as section 11 says (the pipes, `2>$`, `&&`, `||`, `&` and jobs), a bound for the `Queue` (D7), glob and `~` (D1), `.cmd` and `.bat` files on Windows (D14; the error says so when one is in `PATH`). Syntax errors other than those of the scan have no position, as for the rest of ConJS.

## 18. Results of S3: pipelines, `2>$`, `&&` and `||` (2026-11-27)

* **Reading** (conjs/include/conjs_shell.se): `|`, `||` and `&&` outside quotes end a word and become words of their own (a part `("o" op)`); `;` and a single `&` are still SyntaxErrors. `2>$ name`, unquoted, is a redirection, as `<$`, `>$` and `>>$` are. The words are split first at `&&` and `||` (`Conjs_shell_split`), then each part at `|`. `<$` may only be on the first command of a pipeline, `>$` and `>>$` on the last, and `2>$` on external commands; anything else is a SyntaxError at the line.
* **What a line becomes.** `{ let conjs_command_line; P1 if (conjs_command_line === undefined) { P2 } if (conjs_command_line !== undefined) { P3 } ... if (conjs_command_line !== undefined) throw conjs_command_line; }`, with `===` after `&&` and `!==` after `||`: each `P` puts the result of its pipeline (undefined, or an `Error` to throw) into `conjs_command_line`, and only the last that ran throws, as with `set -e` of a shell. The redirections of a pipeline (`v = new Queue();`) are inside its `P`, so they are made only when it runs. A pipeline of one external command is `conjs_sh_run` with the queue of `2>$`; one of a command function is the call on the line, as in S1b; a longer one is `conjs_sh_pipe([{ args: [...], err: e }, { f: name, args: [...] }, ...], input, output, "text")`.
* **`conjs_sh_pipe`** (ConJS, added at the end of the file): the external commands next to each other form a group, which the host runs as one pipeline of the OS; a command function is a group of its own. Between groups is a `Queue` with a limit of 64 (`new Queue(undefined, 64)`; D7): a `push` that fills it returns a Promise, so a function that writes waits, and the host pauses the standard output of a command until the queue has room. Each group runs in a task; when a group ends, the queue it read from is abandoned (`abandon()` of `rt.Queue`: closed, emptied, and a later `push` throws `Queue: the reader has gone`), so that the group before it stops: a function's `push` throws, which `conjs_sh_pipe` takes as a quiet end, as a command that gets SIGPIPE, and the host stops reading the output of a command, which then gets SIGPIPE on its next write. The result is that of the last group, as in Bash without `pipefail` (`false | true` succeeds); an `Error` of a group before it (a command that cannot start, an exception of a function) is written to the standard error and not thrown (below).
* **The host** (vm/wasm/host/shell.js): `conma.sh.run` takes an array of commands as well as one; it starts them with `child_process.spawn`, giving each the standard output of the one before it as its standard input, and lets go of its own copy of that pipe so that SIGPIPE works. The standard error of a command with `2>$` goes line by line into its queue, as the standard output does.
* **Following Bash** (M1's review of shima3/conma#97, and the user's answer: follow Bash, unless that is hard or brings other problems). M1 ran the same examples in Bash (`set -e`, a function that writes `boom` to the standard error and returns 1, the nearest thing to an exception) and found three differences, which this version removes:
    * **An exception of a command function is the failure of that command**, also when it is alone on the line (`try { f(...) } catch (e) { conjs_command_line = e; }` on the line), so `$ $boom || echo fallback` runs `echo fallback`, as a failing function in Bash does. The first version let it out of the line at once, and so `||` took it only in a pipeline.
    * **The result of a pipeline is that of its last command** for exceptions too: `$ $boom | cat || echo fallback` does not run `echo fallback`, as Bash without `pipefail` does not; and a command that cannot start before the last one (`$ no-such-cmd | cat`) is written to the standard error and the pipeline goes on with an empty output (vm/wasm/host/shell.js), as in Bash.
    * **What is not thrown is written to the standard error**: an `Error` that `||` takes and one before the last command of a pipeline are written as `conjs: message` (`conjs_sh_report`, with the built-in `conjs_sh_stderr` of conjs/include/conjs_rt.se), as Bash writes `command not found` and a function writes its own message; a `CommandError` is not, since its command has written to the standard error itself. This is the other problem M1 pointed out: following Bash, an exception that is not thrown would otherwise leave nothing behind.
    * **Kept as it is**: a failure that ends a line throws (`set -e`), so a single `$ $boom` or `$ false` stops the program unless caught, as a script with `set -e` stops; and `pipefail` is not offered yet.
* **The test**: conjs/test/step26 (docs/dev/CONJS.md); step25's case3 is now a `;`.
* **Not done**: `;` and `&` (S4), `pipefail` (a failure before the last command is not seen), `2>&1`, and redirections to files.

## 19. Using S1 to S3: three examples (2026-11-27)

D11 left S4 until S1 to S3 are used. At the user's request, M2 wrote three examples of what scripts do, each with a Bash version that prints the same (conjs/examples; conjs/test/shell-examples/test.sh compares both with one log): logreport.con.js (a summary of a log, with pipelines of external commands and ConJS functions), todos.con.js (a loop over files, with commands that fail, `||`, `&&` and `catch`) and wordcounts.con.js (commands run at the same time in tasks). What writing them showed:

* **The clearest gain**: between command functions, the records stay objects (`{ time, method, path, status, ms }`), so logreport cuts the fields once, where the Bash version cuts them again with `awk` at each step. Running commands at the same time with `spawn` and gathering their results in one `Queue` (wordcounts) is shorter and safer than Bash's `&`, `wait` and a directory of temporary files.
* **Not missed**: `&` (the tasks of `spawn` do it, and give values back with `join`), `;` (a ConJS program writes one statement a line) and job control (no script needs it). This supports D11: S4 can stay where it is.
* **What was awkward**:
    * **The output of a command as one value.** Bash's `$( )` has no counterpart: `let n; $ grep -c TODO ${f} >$ n` then `Number([...n][0])`, and `[...out][0].trim()` for `wc`. A way to take the text of a command as a value (D9 left the command line as an expression for later; or a method of `Queue` such as `text()`) would help most.
    * **`<$` takes a variable only** (D12): a value of ConJS made on the spot, such as `fs.readLines(log)` or an array literal, has to be put into a variable first. The lines of a file need no variable: `$ cat ${log} | $fields | ...` does the same, as the second pipeline of logreport does (logreport's first one uses `<$ lines` only to show a generator given to a command).
    * **Padding of numbers differs between systems** (`wc`, `uniq -c` of GNU and BSD): both versions trim or reprint them, as any shell script that runs on both must.
* **The order of output** from a command and from `console.log` is kept (the program writes synchronously, and a command line waits for its command).

