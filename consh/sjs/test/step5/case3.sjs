// a rejection without a handler ends the program with status 2 (Node.js prints its own message and exits with 1)
async function f() { await 1; throw "unhandled in async" }
f()
console.log("before the end")
