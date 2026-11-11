// 受け手のない例外：投げた文字列
console.log("before")
const f = () => { throw "uncaught" }
f()
console.log("not reached")
