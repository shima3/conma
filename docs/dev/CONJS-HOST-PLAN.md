# Running ConJS on the host's JavaScript

The plan of running the functions of a ConJS program on the JavaScript engine of the host (Node.js or a browser) when the program runs on the wasm VM, calling back into the VM for what the host cannot do. Written on 2026-11-24 at the user's request, after the ConJS benchmarks (conjs/bench/README-ja.md, shima3/conma#71). The first version, the same day, ran only the functions that never wait and undid a call that met anything else; the user then asked that the host call ConJS back instead, and this version does that (3.1), keeping the undoing only as the last resort.

**State**: a plan. Stage H0 (measuring JavaScript written by hand) is done (section 11), and stage H1 (the nested run of the VM) is built (section 12). The rest is not. Section 10 lists what the user is to decide.

## 1. Why

* On the wasm VM, ConJS runs 53 (exceptions) to 18,500 (nbody_class) times slower than Node.js running the same program (conjs/bench/README-ja.md). Every step of ConJS goes through the ConMa VM, which is itself WebAssembly run by the engine of the host.
* The functions of SJS and ConJS that use only numbers and arrays of numbers already run as WebAssembly on the wasm VM (docs/consh/dev/SJS-WASM-PLAN-ja.md and SJS-WASM-ARRAY-PLAN-ja.md): `nbody` of sjs/bench, written with arrays, runs 200 times faster with them (0.55 s against 109 s, M4). They cover little of what ConJS programs do: strings, objects, classes, `map` and `sort`, JSON and exceptions are all outside them, and none of the seven new benchmarks gets a numeric function.
* The host has a JavaScript engine with a JIT that runs all of these. ConJS differs from JavaScript only in a few things (section 5), and most of a ConJS program is plain JavaScript.
* SJS ran on the host's JavaScript once (stage A of docs/consh/dev/SAFEJS-PLAN-ja.md) and left it for two reasons: no sandbox for the scripts of ConSh, and different behaviour from the native VMs. Neither stops this plan: a ConJS program already reads and writes files and the network, and section 7 keeps the results the same as on the native VMs.

## 2. What runs on the host

* **Host functions.** The translator writes a JavaScript version of a function (a declaration, a function expression, an arrow function or a method of a class) when its body uses only what section 4 can write. What it calls need not run on the host: what the host has no version of is called back in the VM (3.1).
* **Two kinds.**
    * A **plain host function** is a JavaScript function. The translator makes one of a function that, as far as it can see, does not wait: every call in it is to another host function, to a built-in that does not wait (`push`, `slice`, `map`, `JSON.stringify`, `Math.sqrt`, `console.log`, ...), or to a method or a function value whose callee it does not know.
    * A **waiting host function** is a JavaScript generator (`function*`). The translator makes one of a function that calls something that waits (`join`, `sleep`, `host.wait`, `fetch`, `readLine`, `nextEvent`, `accept`, the functions of the library that call them), or calls a waiting host function. At each of these calls it yields to the VM (3.1).
* **What the host does not write**: what section 4 cannot write (a generator of ConJS with `yield`, `import` at the top level, constructs to be added later). A function with one stays in ConMa, and the host calls it back like any other function of the VM.
* **Where it starts.** When ConMa code calls a host function, the call runs on the host, with what it calls; calls between host functions stay on the host. Code at the top level of a file runs in ConMa until a later stage (H6) moves it to the host.
* **Only on the wasm VM.** The Rust, transpiled and Python VMs have no JavaScript: there every function is its ConMa version, as now. The program prints the same on every VM (section 7).

## 3. Calling between the VM and the host

### 3.1. What the host does when it cannot go on

A call of a host function begins as in the first version: its arguments (and `this` for a method) are copied from the VM to the host (3.2), and the JavaScript runs on the copies. When it comes to something the host has no version of, it does one of three things.

1. **Call back (plain and waiting host functions).** For a function of the VM that the host has no version of (a method of a class with no host version, a function passed in from ConMa code, a function of the library written in ConMa), or for a value of a kind the host does not have (a task, a file object: reading its member or calling its method), the host calls the VM at once and waits for the result, as a JavaScript function waits for a function it calls:
    1. the host writes back into the VM the copies it changed (3.2) and writes out what `console.log` put in its buffer;
    2. the VM runs the call to its end, as a nested run (8), and returns the value or the exception;
    3. the host copies again from the VM what the call may have changed (3.2), and goes on.
   The call back must not wait: the host's JavaScript cannot be suspended in the middle of a plain function. If the code of the VM comes to a call that waits during a call back, the VM stops it and reports an escape to the host (3, below).
2. **Yield (waiting host functions).** At a call that waits (`join(t)`, `sleep(ms)`, `f.readLine()`, and a call back that may wait), the generator yields a request to the VM:
    1. as before a call back, the host writes back the changed copies and the buffer of `console.log`;
    2. the VProc of the task does the request as ConMa does it now: it waits, and the other tasks run meanwhile;
    3. when the request is done, the VM resumes the generator with the result; the host copies again what may have changed, and goes on.
   So a waiting host function runs as its ConMa version does: other tasks run only where it waits, and see everything it did before that point (the rule of ConJS that a call that waits is the only point where other tasks run).
3. **Escape (the last resort).** A plain host function that comes to a call that waits (a call back whose code waits, a method that turns out to wait) cannot yield. It throws an escape, the host drops its copies and its buffer, and the ConMa version of the function runs from the start with the original arguments, as in the first version of this plan. This undoing is correct only when nothing of the call has been seen outside, so a plain host function that has already called back or written out its buffer cannot escape: the VM stops the program with an error that says the translator chose wrongly (`conjs: internal: a plain host function came to a call that waits after a call back`), to be fixed in the translator. The translator avoids both by making a function a waiting host function whenever it cannot tell (10.1).

### 3.2. Copying values

* **To the host**: numbers, strings, booleans, `undefined` and `null` as they are; arrays as arrays; objects as plain objects with their own properties in the order of the VM; an instance of a class as an object made by the host version of that class (3.3), with its own properties; an error object as an instance of the host version of its error class.
* **What the host keeps as a handle**: functions of the VM (closures), tasks, file objects, servers and requests, generators, the objects of `host`. The host gets a small object that names the value in the VM; calling it, or reading a member of it, calls the VM back (3.1). A handle is passed back to the VM as the value it names.
* **Aliases are kept**: the same array or object (the same `"id"` of the VM) reached twice is copied once, and a cycle (`o.self = o`) is copied as a cycle. The host keeps a table from host objects to the ids of the VM.
* **Back to the VM**: every copied array and object is compared with what was copied in, and the changed ones are written back (their own properties, their length). Objects the host made are made in the VM. The return value is copied the same way.
* **Keeping the two sides the same at a call back or a yield.** The host writes back its changes before the VM runs, and after it copies again the objects it holds. To avoid copying everything again, the VM counts the writes to each array and object (a number kept with it), and the host copies again only those whose count changed.
* **Cost**: the first copy of a call is all that its arguments reach; each call back and yield adds the changed objects. A host function given a large structure for little work, or one that calls back in every step of a loop (`a.map(f)` with `f` a function of the VM), can be slower than its ConMa version. The runtime counts, for each host function, the words copied, the call backs and the time spent, and stops using the host for a function that copies more than it runs (a rule to set when measuring, H5). Copying only what is read (copy on first touch, with proxies) is a later step if the measurements ask for it.

### 3.3. Classes and methods

* Each class with a host function becomes a JavaScript class on the host with the same name, fields and methods. A method with no host version becomes a stub that calls the VM back (3.1).
* `b.move(dt)` on the host calls the method of the object's class on the host, or the stub. If the object has no such method, the host throws the TypeError of ConJS, as the ConMa version does.
* A method read without being called (`const m = o.m`) is bound to `o` as in ConJS (section 5), with a small function of the runtime.

## 4. What the translator writes

* The translator (sjs/include/sjs_gen.se and the files of ConJS) writes, beside the ConMa it writes now, the JavaScript of the host functions: one text for the program, with every host function and class, and a table of their names and kinds (2). The text goes into the module as a String, as the WAT of the numeric functions does, so `--emit` shows it.
* The JavaScript uses only its own names and those of one object of the runtime passed in (`rt`): the translator writes no free name, so the code cannot reach the globals of the page or of Node.js. It is compiled once with `new Function("rt", text)` (Node.js), or loaded from a file made by the build (a browser page; 8).
* What section 5 needs is written as calls of `rt` (`rt.len(s)`, `rt.add(a, b)`), and a call of a method ConJS does not have becomes `rt.missing(...)`, which throws the TypeError of ConJS. Where the translator knows the types (a number in a loop counter, a field written only with numbers), it writes plain JavaScript (`i + 1`).
* Reading and writing a member and calling a method keep the name in the code, with the check of ConJS written in place: `(o == null ? rt.fail(o, "x") : o.x)` and `(typeof o.m === "function" ? o.m(a) : rt.notFn("m"))`, never a call such as `rt.get(o, "x")` with the name as an argument. H0 found the second 25 times slower than plain JavaScript and the first 1.14 times (section 11): with the name in the code, the engine keeps the shape of the object in its caches.
* In a waiting host function, a call that waits becomes `yield rt.wait(...)`, and a call of another waiting host function `yield* g(...)`.
* The ConMa version of each host function stays in the module, wrapped by a function of the runtime (`Sjs_host_fn`, like `Sjs_wasm_fn`) that decides at each call whether to go to the host.

## 5. Keeping the meaning of ConJS

What ConJS does differently from JavaScript (conjs/README.md, Language, and sjs/README-ja.md) is written in the JavaScript of the host, so that a host function gives what its ConMa version gives:

* **Strings count characters** (code points), not units of UTF-16: `length`, indexes, `at`, `slice`, `indexOf`, `replace` and the others go through `rt`, which uses the methods of the host directly when the string has no character outside the Basic Multilingual Plane (checked once per string and kept).
* **`+` joins numbers only**: `rt.add` throws the TypeError of ConJS for anything else, unless the translator knows both sides are numbers.
* **Methods bound when read**, no prototype, `instanceof` only with classes and the errors: written with `rt`.
* **Errors**: the TypeErrors the runtime of ConJS throws (`null.x` and the like) are thrown by `rt` with the same messages; the position of an exception that leaves a host function is that of the ConMa version (the host keeps a table from its lines to the positions of the source). An exception crosses between the host and the VM, both ways, as an error object copied as in 3.2.
* **`console.log`**: formatted on the host by a port of the formatter of the runtime (sjs_rt.se), and compared with it in the tests. It writes into a buffer, written out at the end of the call and before each call back and yield (3.1), so that the output keeps its order with what the VM prints.
* **Numbers**: both are IEEE doubles; `-0` follows JavaScript already (section 11 of SJS-WASM-PLAN-ja.md). Conversions to strings (`String(1e21)`, `toFixed`) use the formatter of the runtime where the host's could differ.
* **Sorting**: the host's `sort` and that of the runtime need not call the comparing function in the same order (to be checked in H2), so the host uses a port of the runtime's algorithm.
* **Missing built-ins**: a method or function that ConJS does not have (`splice`, `Array.from`) is not called on the host, where it exists, but through `rt.missing`, so the program fails in the same way.
* **Tasks**: `spawn(f)` from a host function is a call back: the VM makes the task, and the host gets a handle (3.2). The task runs in the VM when the caller next waits, as now; if `f` has a host version, the task calls it on the host like any other ConMa code.

## 6. Fuel and Ctrl-C

* Code on the host runs until it returns, yields or calls back: the scheduler of the VM, the other tasks and Ctrl-C wait meanwhile. Inside ConJS this changes nothing (code between two calls that wait already runs without interruption), but a long run keeps the page or the terminal from answering.
* The translator puts a counter of fuel at the start of every loop and of every function that can recurse, as for WebAssembly (section 5.5 of SJS-WASM-PLAN-ja.md).
    * **In a waiting host function**, fuel that runs out makes it yield a request to end the time slice: the VM runs the other VProcs, and Ctrl-C comes, and then resumes it with new fuel. Nothing is done again.
    * **In a plain host function**, which cannot yield, fuel that runs out is an escape (3.1, 3) when the call has not called back nor written out its buffer: the runtime gives the time slice back to the VM and calls again with twice the fuel, so the work done again is at most that of the last call, and past a limit the function runs in ConMa, where Ctrl-C stops it at once. When it has called back, the call goes on without fuel until it ends. The translator makes a function with a loop that can be long (a loop whose end it cannot see, a recursion) a waiting host function when it also calls back (10.1).

## 7. The same results on every VM

* A host function runs only on the wasm VM. With `CONMA_NO_HOSTJS=1`, the wasm VM runs the ConMa version (as `CONMA_NO_WASM=1` does for the numeric functions), so that the two can be compared.
* The tests: conjs/test and sjs/test on the wasm VM with and without it, against the Rust VM; a new step of conjs/test with the cases of sections 3 and 5 (aliases and cycles; a call back that changes an object the host holds; `console.log` on both sides of a call back, in order; a yield where another task changes a shared object; an escape; a method that is not a host function; a task in an argument; strings with emoji; exceptions crossing both ways, with their positions; fuel that runs out in both kinds); conjs/test/compare.sh against Node.js; the benchmarks of conjs/bench and sjs/bench.

## 8. The host, the VM and the browser

* **New primitives of the platform feature `"js"`** (docs/PRIMITIVES.md): `__JS_fn_compile__ onError text ,(module)` and `__JS_fn_call__ onError module name args fuel ,(status value)`, made like `__Wasm_compile__` and `__Wasm_call__`. The status tells the VM whether the call returned, threw, escaped, or yielded a request (with the generator to resume, by a handle). On the native VMs they call `onError` (`JavaScript is not available`), and the ConMa version runs. `__JS_call__` is not used: it runs a call as a Stream that the VProc waits for, which is too slow for this.
* **Calling the VM back (a nested run).** A call back happens while the VM is inside the import that called the host. The wasm VM exports a function that runs a closure of the VM with arguments to its end and returns its value: it runs a new VProc for the call, alone, inside the current step, with the scheduler of the VM set aside, and stops it with an escape if it waits. The Rust VM keeps its state in `RefCell`s that the step that called the host still borrows, so it has to be checked, and changed where needed, that a nested run borrows nothing the outer step holds (H1 does this first).
* **Resuming a generator.** The VM keeps the generator of a waiting host function by a handle, as a value of the VProc that waits, and resumes it with `__JS_fn_resume__ onError handle value ,(status value)`, with the same statuses as `__JS_fn_call__`.
* **New imports of vm/wasm/src/sys.rs**: compile a text into a module of host functions, call one, resume a generator, read the result, free a module or a generator. The arguments, the results and the copies of 3.2 cross as one buffer in a compact form of the graph of values, written and read by Rust in the VM and by vm/wasm/conma.js on the host.
* **Content-Security-Policy**: the pages of ConSh (consh/js) forbid `eval` and `Function` when `js` is not used (section 5 of SAFEJS-PLAN-ja.md). For the pages of ConJS, conjs/web/build.sh writes the JavaScript of the host functions into a file of its own, which the page loads with a `<script>`, and which registers its modules by the hash of their text; `__JS_fn_compile__` finds the module by that hash instead of compiling the text. A page that allows `eval` can skip this. Node.js (vm-node.js) uses `new Function`.

## 9. Stages

* **H0 — measure first.** Write by hand the JavaScript the translator would write for `advance` of conjs/bench/nbody_class.con.js and for the loop of hof (moved into a function), with the copies of 3.2 done by a small script, and measure them against Node.js and the wasm VM; measure also a call back in each step of a loop. This tells how much copying, call backs and the calls of `rt` cost before anything is built, and whether the plan is worth it.
* **H1 — the nested run of the VM.** The exported function of 8 that runs a closure to its end inside a step, with the checks of the `RefCell`s, and its stop with an escape when it waits. Tests call it from JavaScript written by hand.
* **H2 — the host and the VM.** The primitives and imports of 8, the copying of 3.2 with aliases, cycles, handles and the counts of writes, call backs, escapes, and the generators and their requests (3.1), still without the translator.
* **H3 — the translator, numbers and strings.** The two kinds of section 2 and the JavaScript for functions of numbers, strings, booleans, arrays and plain objects, with the calls of `rt` for section 5; `console.log` into the buffer.
* **H4 — classes, exceptions and waiting.** Classes and methods (3.3), `throw`, `try`, `catch`, `finally`, `instanceof`, the positions of exceptions; waiting host functions with `yield rt.wait(...)`, and `spawn`.
* **H5 — fuel, then measure and tune.** Section 6; the benchmarks of conjs/bench and sjs/bench; the rules that stop using the host for a function (3.2); the choice between WebAssembly and the host for the numeric functions (both work for them; keep the faster).
* **H6 — the top level.** Running the top level of a file on the host, as a waiting host function.
* **H7 — the browser.** conjs/web/build.sh and the pages (8).

## 10. What the user is to decide

1. **When a function is a waiting host function** (H0: a generator for `advance` alone costs 4%, generators for every function 8.3 times; section 11). A generator is slower than a plain function in the engines, but only it can yield and run long with fuel after a call back. The plan makes a function a waiting host function when it calls something that waits, or when it calls back something the translator does not know and has a loop that can be long; otherwise a plain one. The other choice is to make every host function that calls back a generator, which never needs the error of 3.1, 3, at some cost in speed.
2. **Whether to go on after H0** (section 11): the host's JavaScript is 1.1 to 1.8 times Node.js where the VM is 1,000 to 18,500 times, but the crossings of today are too slow (a call back costs about 18 µs, and copying a value about 4 µs), so H1 and H2 must make them faster, with the copying done by Rust in the VM.
3. **The browser (8)**: a file of JavaScript made by the build, or allowing `eval` in the pages of ConJS.
4. **The numeric functions**: keep WebAssembly for them, or move them to the host when the host is as fast (H5).
5. **SJS**: whether SJS in ConSh (`(( ))`) uses host functions too. Its scripts are not trusted in a page in the way a ConJS program is, so the plan leaves it out unless the user says otherwise.

## 11. Results of H0 (2026-11-24)

Measured on the M4 (Apple M4), Node.js v26.13.0, user + sys, the least of 3 runs, at commit 0ffa27a with the scripts of conjs/bench/hostjs (load average about 2.3 from other sessions). The scripts print the same energy and sums as the programs of ConJS.

**The host's JavaScript, written by hand as the translator would write it** (conjs/bench/hostjs/nbody_host.js for `advance` of conjs/bench/nbody_class.con.js, 20,000,000 steps; hof_host.js for the loop of conjs/bench/hof.con.js of shima3/conma#71, moved into a function `round(r)`, 350,000 rounds):

| What | nbody | hof |
|---|---:|---:|
| Node.js, the program as it is | 1.02 s | 1.03 s |
| with a counter of fuel in each loop (6) | 1.04 s | 1.04 s |
| with the checks of ConJS written in place, names in the code (4) | 1.21 s | — |
| with `+` through `rt.add` and the merge sort of the runtime (5) | — | 1.81 s |
| fields through `rt.get(o, "x")` and `rt.set`, methods direct | 26.2 s | — |
| methods through `rt.call(o, "m", ...args)`, fields direct | 3.71 s | — |
| both through `rt` | 28.3 s | — |
| `advance` a generator (a waiting host function) | 1.09 s | 1.07 s |
| `advance`, `pull` and `move` all generators, with `yield*` | 8.69 s | — |
| the wasm VM, ConJS (conjs/bench/README-ja.md of #71; per step and per round) | 1.0 ms | 5.4 ms |
| the host, per step and per round (best form above) | 60 ns | 5.2 µs |

* The best forms are 1.1 (nbody) and 1.8 (hof) times Node.js, where ConJS on the wasm VM is 18,500 and 1,800 times. Moved to the host, `advance` would run about 16,000 times faster, and a round of hof about 1,000 times.
* Writing the checks with the names in the code is what keeps the speed: `rt.get(o, "x")` makes the engine look the name up each time and is 25 times slower (section 4 now says so). A call through `rt.call` with the arguments spread is 3.6 times slower.
* Fuel costs 2 to 4%. A generator for the function that waits costs 4%; generators for every function cost 8.3 times, so only the functions that need it are to be generators (10.1).
* Copying the system of nbody (5 bodies, 10 pairs, about 60 values) from a list of nodes like the buffer of section 8 into objects of the host and back, around each step: 2.2 µs a step on the host side (`copy`, 1,000,000 steps, 2.24 s against 0.10 s without copying).

**Crossings of today on the wasm VM** (conjs/bench/hostjs/crossing.con.js, ConJS on the wasm VM; the time per step after taking out an empty loop):

| What | per step |
|---|---:|
| an empty loop | 3.6 µs |
| a call of a small ConJS function (the work of a call back in the VM) | 6.8 µs |
| `host.call("test.add", i, 1)`: to the host and back with numbers | 10.7 µs |
| `host.call("test.echo", system)`: to the host and back with an object of about 45 values | 386 µs |

* A call back would cost about the crossing and the call, some 18 µs, against 6.8 µs for the same call made in the VM. So a host function that calls back in every step of a loop (`a.map(f)` with `f` in the VM) is slower than its ConMa version, as 3.2 expected: the rule that stops using the host for such a function is needed (H5).
* Copying values by the path of `host.call` today costs about 4 µs a value each way, 2,000 times what the host spends on its side. That path converts the values in ConMa code; the copying of 3.2 must be done by Rust in the VM (a primitive that writes a graph of Dicts into the buffer and reads it back), as the copying of numeric arrays was made a primitive for WebAssembly (section 17 of SJS-WASM-ARRAY-PLAN-ja.md). H2 measures it.
* For nbody, `system.advance(0.01, n)` is called once: one copy each way (under 1 ms even at today's 4 µs a value) and 20,000,000 steps on the host. For hof moved into a function called from a loop at the top level, each round costs a crossing (about 11 µs) besides its 5.2 µs, so the top level should go to the host too (H6), or the round would gain about 340 times instead of 1,000.

## 12. Results of H1 (2026-11-24)

The nested run of the VM (8) is built, for the global functions of a program, without the translator.

* **The VM** (vm/src/prim/host.rs, `callback`): while `__Host_call__` runs a function of the host, it keeps what a call back needs of the VProc that called (its global variables, its PDict, its current directory, the Booleans of its Machine). A call back runs the function in a Machine of its own (vm/src/sched.rs), with a VProc that gets that PDict, until it ends; the VProc that called gets the PDict back with what the call back changed. Nothing else of the VM runs meanwhile.
    * The collector does not run during a nested run (vm/src/gc.rs, `due`): the step that called the host still borrows its VProc, which the collector would borrow too. The `RefCell`s were the worry of 8: running a new Machine borrows nothing that the outer step holds, as the outer Machine is not touched and the VProc that called is not reached.
    * A call back that comes to something that waits ends as an escape (3.1): with no VProc ready, the Machine of the call back returns instead of reporting a deadlock (`nested` of `Machine`). One that leaves a VProc it started unfinished is an escape too. An error of the function ends it as an error.
* **The wasm VM** (vm/wasm/src/lib.rs): `conma_callback(name, args)` calls the global function `name` with the arguments in the JSON of `__Host_call__`, and gives `{"r":value}`, `{"e":"message"}` or `{"x":"message"}` (an escape).
* **The host** (vm/wasm/conma.js): a function of the host that `__Host_call__` calls gets `this.callVm(name, ...args)`, which returns the value, or throws an Error (with `escape` true for an escape). Arrow functions do not get it.
* **The test**: test/fast19 (test/run_all.sh runs it). Its functions of the host (host.js) call back a function that returns a number, one that returns a Dict with an LList, one that calls the host again (two nested runs), one that stops with an error, one that waits (an escape), a name that is not there, and one that writes the PDict, which the program reads after the call. The other VMs have no such functions of the host (case1.log).
* **What a call back costs** (the M4, the wasm VM, user + sys, the least of 3): a function of the host that calls back `twice` 100,000 times takes 0.484 s, against 0.154 s for the same loop in ConMa, after taking out the start (0.094 s and 0.08 s for 1,000): **3.9 µs a call back**, against 0.75 µs for a call and a step of the loop in the VM. H0 expected about 18 µs (a crossing of `host.call` and a call); the JSON of the arguments and the result, and a new Machine for each call back, are most of it, to be measured again with the copying of H2.
* **Not done**: calling back a closure that is not a global variable (a handle, 3.2; H2), copying with aliases (H2), and the native VMs, which have no host that runs during `__Host_call__`.

