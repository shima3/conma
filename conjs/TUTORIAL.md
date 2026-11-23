**English** | [日本語](TUTORIAL-ja.md)

# ConJS Tutorial

The basic JavaScript that ConJS takes, and where ConJS differs from JavaScript, by examples. It is written for readers who know a little JavaScript, but a beginner can follow it too, typing and running the examples. If ConJS does not run on your computer yet, go through the [quick start](QUICKSTART.md) first.

Write each example in a file (`example.con.js`) in the folder `conma`, and run it:

```
$ conjs/conjs example.con.js
```

What follows `Output` under an example is what it prints. You may delete the files the examples make (`example.con.js`, `memo.txt`, `lib/` and others) when you no longer need them.

## 1. What ConJS is

ConJS is a subset of JavaScript. Most ConJS programs also run the same as JavaScript. They differ in two ways:

* **Easy mistakes are errors**: `+` of values of different types, such as `"age" + 20`, and `==` cannot be used, so such mistakes show up as soon as the program runs.
* **A call that waits is an ordinary call**: what JavaScript writes with `async` and `await` (waiting for a time, for the answer of a request, and so on) is written as an ordinary function call. There is no `async`, `await` or `Promise`.

Sections 2 to 6 are written as in JavaScript; sections 7 to 9 are where ConJS differs.

## 2. Values and variables

```js
const name = "ConJS";
let count = 3;
count = count + 1;
console.log(`Hello, ${name}! count is ${count}`);
console.log(7 / 2, 7 % 2, 2 ** 10, Math.floor(3.7));
console.log(typeof name, typeof count, typeof true, typeof undefined);
```

Output:
```
Hello, ConJS! count is 4
3.5 1 1024 3
string number boolean undefined
```

* Make a variable with `const` (not changed) or `let` (changed). `var` works too, but use `let` and `const`.
* To put a value in a string, use a **template literal** (between `` ` ``, with `${...}` for a value). In ConJS this is how strings are joined (7.1).
* Integers and fractions are both numbers. `Math` has `floor`, `ceil`, `round`, `abs`, `sqrt`, `max`, `min`, `PI` and others.

## 3. Conditions and loops

```js
for (let i = 1; i <= 15; i++) {
  if (i % 15 === 0) console.log("FizzBuzz");
  else if (i % 3 === 0) console.log("Fizz");
  else if (i % 5 === 0) console.log("Buzz");
  else console.log(i);
}
let n = 0;
while (n < 3) n++;
console.log(`n = ${n}`);
```

Output (the beginning and the end):
```
1
2
Fizz
…
14
FizzBuzz
n = 3
```

`if`, `else`, `for`, `while`, `do ... while`, `switch`, `break` and `continue` are as in JavaScript. Compare with `===` for equal and `!==` for not equal (7.2).

## 4. Arrays and objects

```js
const scores = [72, 95, 58, 81];
scores.push(66);
console.log(scores.length, scores[0], scores[scores.length - 1]);
const passed = scores.filter((s) => s >= 60);
const doubled = scores.map((s) => s * 2);
const total = scores.reduce((sum, s) => sum + s, 0);
console.log(passed, doubled.slice(0, 2), total);
console.log([...scores].sort((a, b) => a - b).join(", "));
for (const s of scores) if (s > 90) console.log(`high score: ${s}`);

const student = { name: "Hanako", year: 2, courses: ["math", "English"] };
student.year = 3;
console.log(student.name, student.courses[1], Object.keys(student));
const { name, year } = student;
console.log(`${name} is in year ${year}`);
console.log(JSON.stringify(student));
```

Output:
```
5 72 66
[ 72, 95, 81, 66 ] [ 144, 190 ] 372
58, 66, 72, 81, 95
high score: 95
Hanako English [ 'name', 'year', 'courses' ]
Hanako is in year 3
{"name":"Hanako","year":3,"courses":["math","English"]}
```

* `push`, `map`, `filter`, `reduce`, `slice`, `sort`, `join`, `includes` and `indexOf` of arrays, and `for ... of`, are as in JavaScript.
* `[...scores]` is a spread (a new array with the same elements), and `const { name, year } = student` a destructuring.
* `JSON.stringify` and `JSON.parse` are there too.
* Some newer methods of JavaScript are not there yet (such as `at` of arrays). Calling a missing method gives `TypeError: ... is not a function` when it runs.

## 5. Functions

```js
function area(w, h = w) {
  return w * h;
}
const square = (x) => x * x;
console.log(area(3, 4), area(5), square(9));

function sum(...xs) { return xs.reduce((a, b) => a + b, 0); }
console.log(sum(1, 2, 3, 4));

function makeCounter() {
  let c = 0;
  return () => ++c;
}
const next = makeCounter();
next(); next();
console.log(next());
```

Output:
```
12 25 81
10
3
```

Function declarations, arrow functions (`(x) => x * x`), default values of parameters (`h = w`), rest parameters (`...xs`) and closures (a function that keeps the `c` of `makeCounter`) are as in JavaScript.

## 6. Classes and exceptions

```js
class Point {
  constructor(x, y) { this.x = x; this.y = y; }
  length() { return Math.sqrt(this.x ** 2 + this.y ** 2); }
}
class Point3 extends Point {
  constructor(x, y, z) { super(x, y); this.z = z; }
}
const p = new Point(3, 4);
console.log(p, p.length());
console.log(new Point3(1, 2, 3) instanceof Point);

try {
  throw new RangeError("out of range");
} catch (e) {
  console.log(`${e.name}: ${e.message}`);
} finally {
  console.log("done");
}
```

Output:
```
Point { x: 3, y: 4 } 5
true
RangeError: out of range
done
```

`class`, `constructor`, methods, `extends`, `super`, `new`, `instanceof`, `try`, `catch`, `finally` and `throw` are as in JavaScript. The examples of 2 to 6 also run in Node.js as they are, with the same output (try `node example.con.js`).

## 7. Where ConJS differs: what is an error

ConJS makes errors of ways of writing that JavaScript runs but that are easy mistakes. There are two kinds of errors:

* **Errors when it is translated** (beginning `conjs:`): found before the program runs.
* **Errors when it runs** (beginning `file:line:column:`): found when that line runs.

### 7.1. `+` takes numbers only

```js
const age = 20;
console.log("age: " + age);
```

Output:
```
example.con.js:2:13: TypeError: + needs numbers, not the string "age: " (join strings with a template literal)
```

JavaScript gives `"age: 20"`, but as `"1" + 2` giving `"12"` is a common mistake, `+` in ConJS takes numbers only. Join strings with a template literal:

```js
console.log(`age: ${age}`);
```

### 7.2. No `==` and `!=`

```js
console.log(1 == 1);
```

Output:
```
conjs: not in ConJS: == (use ===)
```

`==` converts values of different types before comparing them (`0 == ""` is true), so ConJS does not have it. Use `===` and `!==`.

### 7.3. `<` and the like compare two numbers or two strings

```js
console.log(1 < "2");
```

Output:
```
example.con.js:1:13: TypeError: < compares two numbers or two strings, not the number 1 and the string "2"
```

### 7.4. What is not there

| JavaScript | In ConJS |
|---|---|
| Regular expressions (`/-/g`) | `conjs: not in ConJS: a regular expression`. Use the methods of strings (`split`, `join`, `includes`, `indexOf`, `slice` and others); a replacement can be written `s.split("-").join("+")` (`replace` and `replaceAll` are not there yet) |
| `Set`, `Map` | `ReferenceError: Set is not defined`. Use objects and arrays |
| `this` outside a class | `conjs: this outside a class`. `this` is only for classes |
| `Math.sin` and others | Some functions of `Math` are not there yet |
| `async`, `await`, `Promise` | `conjs: not in ConJS: async`. Write a wait as in section 8 |

### 7.5. The length of a string is its number of characters

```js
console.log("ConJS".length, "日本語".length, "😀".length);
```

Output (ConJS):
```
5 3 1
```

JavaScript (Node.js) gives `5 3 2`: it counts units of UTF-16, two for the emoji, while ConJS counts characters.

## 8. Where ConJS differs: calls that wait

This is what ConJS is for.

### 8.1. `sleep`: wait for a time

```js
console.log("counting 3 seconds");
for (let i = 3; i > 0; i--) {
  console.log(i);
  sleep(1000);
}
console.log("go!");
```

Output (a line each second):
```
counting 3 seconds
3
2
1
go!
```

`sleep(ms)` returns after ms milliseconds. In JavaScript, you make the waiting function with a Promise and call it with `await`:

```js
// JavaScript (Node.js)
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
console.log("counting 3 seconds");
for (let i = 3; i > 0; i--) {
  console.log(i);
  await sleep(1000);   // without await, it would go on without waiting
}
console.log("go!");
```

In JavaScript, every function that uses `await` must be `async`, and every call of it needs `await` too. ConJS has no `await`: an ordinary call waits, in any function.

### 8.2. `spawn` and `join`: run several tasks at once

```js
function worker(name, ms) {
  for (let i = 1; i <= 3; i++) {
    sleep(ms);
    console.log(`${name} ${i}`);
  }
  return `${name} done`;
}
const a = spawn(() => worker("fast", 100));
const b = spawn(() => worker("slow", 250));
console.log(join(a));
console.log(join(b));
```

Output:
```
fast 1
fast 2
slow 1
fast 3
fast done
slow 2
slow 3
slow done
```

* `spawn(f)` starts the function f as a new **task** (a lightweight thread) and returns the task. The caller goes on.
* `join(t)` waits until the task t ends and returns what f returned.
* While a task waits (in `sleep` and the like), the others run. Tasks switch only at calls that wait (`sleep`, `join`, the calls that wait for keys or files, and so on).

In JavaScript, `worker` would be an `async function`, with `await` before `sleep`, and `await a` instead of `join(a)`.

There is `setTimeout(f, ms)` too. Unlike JavaScript, it returns a task, not a number, so `join` gives what f returned:

```js
const t = setTimeout(() => "2 seconds passed", 2000);
console.log("this runs while it waits");
console.log(join(t));
```

Output:
```
this runs while it waits
2 seconds passed
```

### 8.3. Read a file line by line

```js
fs.writeFile("memo.txt", "apple\norange\ngrape\n");
let n = 0;
for (const line of fs.readLines("memo.txt")) {
  n++;
  console.log(`${n}: ${line}`);
}
console.log(argv);
```

Output, run as `conjs/conjs example.con.js x y`:
```
1: apple
2: orange
3: grape
[ 'x', 'y' ]
```

* `fs` is there from the start (no `require` or `import`). It has `fs.readFile`, `fs.writeFile`, `fs.readLines` and `fs.open`.
* The lines go through an ordinary `for ... of`. Node.js reads them with `readline` and `for await`.
* The arguments given to the program are in `argv` (`process.argv.slice(2)` in Node.js; ConJS has no `process`).

A web server (`http.listen`), requests (`fetch`) and keys (`nextEvent`) are calls that wait in the same way. See the server of the [quick start](QUICKSTART.md) and the [README](README.md).

## 9. Split a program into files: `import` and `export`

A program can be split into files, written as with the ES Modules of JavaScript.

`lib/geometry.con.js`:
```js
export const PI = 3.14159;
export function circleArea(r) { return PI * r * r; }
```

`example.con.js`:
```js
import { circleArea, PI as pi } from "./lib/geometry.con.js";
console.log(pi, circleArea(2));
```

Output:
```
3.14159 12.56636
```

* The path begins with `./` or `../` and ends with the extension (`.con.js`).
* Only named `import { ... }` and `export` are taken; `export default` and `import * as m` are not yet.

## 10. Exercises

1. Write the FizzBuzz of section 3 from 1 to 30 with `while` instead of `for`.
2. Print the average of `scores` of section 4 as "average: ... points" with a template literal (`"average: " + avg` is an error).
3. `spawn` three `worker`s of 8.2 that count at different intervals. Guess the order of the lines before you run it, then check.
4. Add a path `/hello` that answers `hello` to the server of the quick start.

All of the language and the library of ConJS is in the [README](README.md).
