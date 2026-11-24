# Running the parts of ConJS that do not wait on the host JavaScript

The plan of running the functions of a ConJS program that never wait on the JavaScript engine of the host (Node.js or a browser), when the program runs on the wasm VM. Written on 2026-11-24 at the user's request, after the ConJS benchmarks (conjs/bench/README-ja.md, shima3/conma#71).

**State**: a plan. Nothing is built. Section 10 lists what the user is to decide.

## 1. Why

* On the wasm VM, ConJS runs 53 (exceptions) to 18,500 (nbody_class) times slower than Node.js running the same program (conjs/bench/README-ja.md). Every step of ConJS goes through the ConMa VM, which is itself WebAssembly run by the engine of the host.
* The functions of SJS and ConJS that use only numbers and arrays of numbers already run as WebAssembly on the wasm VM (docs/consh/dev/SJS-WASM-PLAN-ja.md and SJS-WASM-ARRAY-PLAN-ja.md): `nbody` of sjs/bench, written with arrays, runs 200 times faster with them (0.55 s against 109 s, M4). They cover little of what ConJS programs do: strings, objects, classes, `map` and `sort`, JSON and exceptions are all outside them, and none of the seven new benchmarks gets a numeric function.
* The host has a JavaScript engine with a JIT that runs all of these. ConJS differs from JavaScript only in a few things (section 5), and most of the code between two calls that wait is plain JavaScript.
* SJS ran on the host's JavaScript once (stage A of docs/consh/dev/SAFEJS-PLAN-ja.md) and left it for two reasons: no sandbox for the scripts of ConSh, and different behaviour from the native VMs. Neither stops this plan: a ConJS program already reads and writes files and the network, and section 7 keeps the results the same as on the native VMs.

## 2. What runs on the host

* **Host functions.** The translator marks a function (a declaration, a function expression, an arrow function or a method of a class) as a host function when its body uses only what section 4 can write in JavaScript and every call in it is to something that does not wait:
    * another host function, bound by a name that is not assigned again (as for the numeric functions, section 5.1 of SJS-WASM-PLAN-ja.md);
    * a method of a built-in value that ConJS has and that does not wait (`push`, `slice`, `map`, `JSON.stringify`, `Math.sqrt`, `console.log`, ...);
    * a method called on an object (`b.move(dt)`), when some class of the program has a host function of that name and no class has a method of that name that is not one (the class of `b` is known only when the call runs; section 3.3).
* **What is never a host function**: a function that waits (`join`, `sleep`, `host.wait`, `fetch`, `readLine`, `nextEvent`, `accept` and the functions of the library that call them), `spawn`, `host.call`, the functions of `fs`, generators and `yield`, a call of a function value whose origin is not known (a parameter called as a function: `f(x)`), and anything section 4 cannot write.
* **Arguments that are functions.** `a.map(x => x * 2)` with an arrow written there is fine: the arrow becomes part of the host function. A function passed in from outside (`a.map(f)` with `f` a parameter) is not, in the first stages.
* **Where it starts.** The ConMa side calls a host function only at the boundary, as with the numeric functions: when ConMa code calls a host function, the whole call, with all it calls, runs on the host. Calls between host functions stay on the host. Code at the top level of a file is not a function, so a loop there runs in ConMa; moving the top level of a file into host code is a later stage (H6).
* **Only on the wasm VM.** The Rust, transpiled and Python VMs have no JavaScript: there the host function is the ConMa version, as now. The program prints the same on every VM (section 7).

## 3. Calling a host function

### 3.1. A call is a transaction

The translator cannot see every case: a method can be called on an object of a class it did not expect, a value can be a task or a file object, a loop can run longer than a time slice allows. So a call of a host function is made so that it can be **undone and run again in ConMa**, as a call of a numeric function in WebAssembly can (section 2 of SJS-WASM-PLAN-ja.md):

1. The arguments (and `this` for a method) are copied from the VM to the host (3.2).
2. The JavaScript runs on the copies. `console.log` writes into a buffer on the host, not to the output.
3. If it ends (with a return value or with an exception), the changes to the copied objects are written back into the VM (3.2), the buffer of `console.log` is written out, and the value or the exception is given to the caller.
4. If it reaches something it cannot do (an **escape**: a method that is not a host function, a value of a kind the host does not have, a missing built-in, fuel that runs out; 3.3 and 6), the host throws an escape, drops the copies and the buffer, and the ConMa version of the same function runs from the start with the original arguments. Nothing the host did is seen.

As in the case of WebAssembly, this holds because a host function touches nothing but its copies and the buffer: it cannot write files, call the host or start tasks (section 2 leaves them out).

### 3.2. Copying values

* **To the host**: numbers, strings, booleans, `undefined` and `null` as they are; arrays as arrays; objects as plain objects with their own properties in the order of the VM; an instance of a class as an object made by the host version of that class (3.3), with its own properties; an error object as an instance of the host version of its error class.
* **Aliases are kept**: the same array or object (the same `"id"` of the VM) reached twice is copied once, and a cycle (`o.self = o`) is copied as a cycle. The host keeps a table from host objects to the ids of the VM.
* **Back to the VM**: every copied array and object is compared with what was copied in, and the changed ones are written back (their own properties, their length). Objects the host made are made in the VM. The return value is copied the same way.
* **What is not copied**: functions (closures), tasks, file objects, servers and requests, generators, the objects of `host`. Meeting one when copying in makes the call run in ConMa at once, before any JavaScript runs.
* **Cost**: a call copies everything it can reach from its arguments, so a host function given a large structure for little work is slower than the ConMa version. The runtime counts, for each host function, the words copied and the time spent, and stops using the host for a function whose calls copy more than they run (a rule to set when measuring, H5). Copying only what is read (copy on first touch, with proxies) is a later step if the measurements ask for it.

### 3.3. Classes and methods

* Each class with a host function becomes a JavaScript class on the host with the same name, fields and methods. Methods that are not host functions become stubs that throw an escape.
* `b.move(dt)` on the host calls the method of the object's class on the host; if it is a stub or the object has no such method, the call escapes (3.1, 4).
* A method read without being called (`const m = o.m`) is bound to `o` as in ConJS (section 5), with a small function of the runtime.

## 4. What the translator writes

* The translator (sjs/include/sjs_gen.se and the files of ConJS) writes, beside the ConMa it writes now, the JavaScript of the host functions: one text for the program, with every host function and class, and a table of their names. The text goes into the module as a String, as the WAT of the numeric functions does, so `--emit` shows it.
* The JavaScript uses only its own names and those of one object of the runtime passed in (`rt`): the translator writes no free name, so the code cannot reach the globals of the page or of Node.js. It is compiled once with `new Function("rt", text)` (Node.js), or loaded from a file made by the build (a browser page; 8).
* What section 5 needs is written as calls of `rt` (`rt.len(s)`, `rt.add(a, b)`, `rt.get(o, "x")`), and a call of a method ConJS does not have becomes `rt.missing(...)`, which throws the TypeError of ConJS. Where the translator knows the types (a number in a loop counter, a field written only with numbers), it writes plain JavaScript (`i + 1`, `o.x`) so that the JIT sees ordinary code.
* The ConMa version of each host function stays in the module, wrapped by a function of the runtime (`Sjs_host_fn`, like `Sjs_wasm_fn`) that decides at each call whether to go to the host.

## 5. Keeping the meaning of ConJS

What ConJS does differently from JavaScript (conjs/README.md, Language, and sjs/README-ja.md) is written in the JavaScript of the host, so that a host function gives what its ConMa version gives:

* **Strings count characters** (code points), not units of UTF-16: `length`, indexes, `at`, `slice`, `indexOf`, `replace` and the others go through `rt`, which uses the methods of the host directly when the string has no character outside the Basic Multilingual Plane (checked once per string and kept).
* **`+` joins numbers only**: `rt.add` throws the TypeError of ConJS for anything else, unless the translator knows both sides are numbers.
* **Methods bound when read**, no prototype, `instanceof` only with classes and the errors: written with `rt`.
* **Errors**: the TypeErrors the runtime of ConJS throws (`null.x` and the like) are thrown by `rt` with the same messages; the position of an exception that leaves a host function is that of the ConMa version (the host keeps a table from its lines to the positions of the source). An exception that leaves the host function is copied back as an error object of the VM.
* **`console.log`**: formatted on the host by a port of the formatter of the runtime (sjs_rt.se), and compared with it in the tests.
* **Numbers**: both are IEEE doubles; `-0` follows JavaScript already (section 11 of SJS-WASM-PLAN-ja.md). Conversions to strings (`String(1e21)`, `toFixed`) use the formatter of the runtime where the host's could differ.
* **Sorting**: the host's `sort` and that of the runtime need not call the comparing function in the same order (to be checked in H2), so the host uses a port of the runtime's algorithm, or a comparing function with side effects (it logs, say) escapes (the translator can see that it has a call of `console.log`).
* **Missing built-ins**: a method or function that ConJS does not have (`splice`, `Array.from`) is not called on the host, where it exists, but through `rt.missing`, so the program fails in the same way.

## 6. Fuel and Ctrl-C

* A call of the host runs until it returns: the scheduler of the VM, the other tasks and Ctrl-C wait. Inside ConJS this changes nothing (code between two calls that wait already runs without interruption), but a long call keeps the page or the terminal from answering.
* As for WebAssembly (section 5.5 of SJS-WASM-PLAN-ja.md), the translator puts a counter of fuel at the start of every loop and of every function that can recurse. When it runs out, the call escapes; the runtime gives the time slice back to the VM and calls again with twice the fuel. The work done again is at most that of the last call. Past a limit, the function runs in ConMa, where Ctrl-C stops it at once.
* A call that is undone wastes its work. The runtime keeps, for each host function, how often it escaped, and stops using the host for one that escapes often.

## 7. The same results on every VM

* A host function runs only on the wasm VM. With `CONMA_NO_HOSTJS=1`, the wasm VM runs the ConMa version (as `CONMA_NO_WASM=1` does for the numeric functions), so that the two can be compared.
* The tests: conjs/test and sjs/test on the wasm VM with and without it, against the Rust VM; a new step of conjs/test with the cases of sections 3 and 5 (aliases and cycles, an escape after `console.log`, a method that is not a host function, a task in an argument, strings with emoji, exceptions with their positions, fuel that runs out); conjs/test/compare.sh against Node.js; the benchmarks of conjs/bench and sjs/bench.

## 8. The host, the VM and the browser

* **A new primitive of the platform feature `"js"`** (docs/PRIMITIVES.md): `__JS_fn_compile__ onError text ,(module)` and `__JS_fn_call__ onError module name args fuel ,(status value)`, made like `__Wasm_compile__` and `__Wasm_call__`. On the native VMs they call `onError` (`JavaScript is not available`), and the ConMa version runs. `__JS_call__` is not used: it runs a call as a Stream that the VProc waits for, which is too slow for this.
* **New imports of vm/wasm/src/sys.rs**: compile a text into a module of host functions, call one with its arguments, read the result, free a module. The arguments and the results cross as one buffer in a compact form of the graph of values (3.2), written and read by Rust in the VM and by vm/wasm/conma.js on the host.
* **Content-Security-Policy**: the pages of ConSh (consh/js) forbid `eval` and `Function` when `js` is not used (section 5 of SAFEJS-PLAN-ja.md). For the pages of ConJS, conjs/web/build.sh writes the JavaScript of the host functions into a file of its own, which the page loads with a `<script>`, and which registers its modules by the hash of their text; `__JS_fn_compile__` finds the module by that hash instead of compiling the text. A page that allows `eval` can skip this. Node.js (vm-node.js) uses `new Function`.

## 9. Stages

* **H0 — measure first.** Write by hand the JavaScript the translator would write for `advance` of conjs/bench/nbody_class.con.js and for the loop of hof (moved into a function), with the copies of 3.2 done by a small script, and measure them against Node.js and the wasm VM. This tells how much copying and the calls of `rt` cost before anything is built, and whether the plan is worth it.
* **H1 — the host and the VM.** The primitives and imports of 8, the copying of 3.2 with aliases and cycles, escapes and the transaction of 3.1, without the translator: tests call them with JavaScript written by hand.
* **H2 — the translator, numbers and strings.** The analysis of section 2 and the JavaScript for functions of numbers, strings, booleans, arrays and plain objects, with the calls of `rt` for section 5; `console.log` into the buffer.
* **H3 — classes and exceptions.** Classes and methods (3.3), `throw`, `try`, `catch`, `finally`, `instanceof`, the positions of exceptions.
* **H4 — fuel.** Section 6.
* **H5 — measure and tune.** The benchmarks of conjs/bench and sjs/bench; the rules that stop using the host for a function (3.2, 6); the choice between WebAssembly and the host for the numeric functions (both work for them; keep the faster).
* **H6 — the top level.** Running a part of the top level of a file that does not wait (a loop) on the host, as if it were a function.
* **H7 — the browser.** conjs/web/build.sh and the pages (8).

## 10. What the user is to decide

1. **Undo and run again (3.1)**, or run only what the translator can prove never escapes. Undoing lets the translator be simple and safe, at the cost of copying the arguments and of the work of an escaped call; proving leaves out every method call whose class is not known, which is most of nbody_class.
2. **Whether H0 comes first** and the rest waits for its numbers (recommended: the copying of 3.2 could eat the gain for small functions).
3. **The browser (8)**: a file of JavaScript made by the build, or allowing `eval` in the pages of ConJS.
4. **The numeric functions**: keep WebAssembly for them, or move them to the host when the host is as fast (H5).
5. **SJS**: whether SJS in ConSh (`(( ))`) uses host functions too. Its scripts are not trusted in a page in the way a ConJS program is, so the plan leaves it out unless the user says otherwise.
