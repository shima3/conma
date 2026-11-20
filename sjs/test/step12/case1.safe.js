// JSON.parse と配列の sort（docs/dev/PLATFORM-PLAN-ja.md の 4.5 節。ConJS と共有する）
console.log(JSON.parse('{"a": [1, 2.5, -3e2, {"b": null}], "c": "x\\ny\\u00e9\\ud83d\\ude00", "d": true, "e": false}'));
console.log(JSON.parse(" [ ] "), JSON.parse("{}"), JSON.parse('"s"'), JSON.parse("42"), JSON.parse("null"));
const data = JSON.parse(JSON.stringify({ list: [3, 1, 2], name: "n", nested: { deep: [true] } }));
console.log(data.list, data.nested.deep[0], Object.keys(data));
for (const bad of ["{bad", "[1,]", "", "tru", '{"a" 1}', "[1 2]"]) {
  try {
    JSON.parse(bad);
    console.log("parsed", bad);
  } catch (e) {
    console.log(e.name, e.name === "SyntaxError");
  }
}

console.log([3, 1, 10, 2, 21].sort());
console.log([3, 1, 10, 2, 21].sort((a, b) => a - b));
console.log([3, 1, 10, 2, 21].sort((a, b) => b - a));
console.log(["b", undefined, "a", "c", undefined].sort());
console.log(["banana", "apple", "Cherry", "apple2", ""].sort());
const people = [{ n: "a", age: 30 }, { n: "b", age: 25 }, { n: "c", age: 30 }, { n: "d", age: 25 }, { n: "e", age: 20 }];
console.log(people.sort((x, y) => x.age - y.age).map((p) => p.n).join(""));
const arr = [5, 4, 3];
console.log(arr.sort() === arr, arr, [].sort(), [1].sort());
const big = [];
for (let i = 0; i < 200; i++) big.push((i * 37) % 101);
const sorted = big.slice().sort((a, b) => a - b);
let ok = true;
for (let i = 1; i < sorted.length; i++) if (sorted[i - 1] > sorted[i]) ok = false;
console.log(ok, sorted.length, sorted[0], sorted[199]);
try {
  [1, 2].sort(1);
} catch (e) {
  console.log(e.name, e.message);
}
