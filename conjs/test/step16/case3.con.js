// readLines() と閉じること（4.8 節、段階 P8）：for を回し切ったとき、break で抜けたとき、本体で例外を投げたとき、ループの
// ある関数を return で抜けたときに、ファイルが閉じられる（closed）。ファイルのオブジェクトの見え方
fs.writeFile("lines.txt", "a\nb\nc\n");
const f1 = fs.open("lines.txt");
for (const line of f1.readLines()) console.log("all", line);
console.log("after all:", f1.closed);
const f2 = fs.open("lines.txt");
for (const line of f2.readLines()) { console.log("break at", line); break; }
console.log("after break:", f2.closed);
const f3 = fs.open("lines.txt");
try {
  for (const line of f3.readLines()) if (line === "b") throw new Error(`stop at ${line}`);
} catch (e) { console.log(`${e}`); }
console.log("after throw:", f3.closed);
const f4 = fs.open("lines.txt");
function find(f, want) { for (const line of f.readLines()) if (line === want) return line; return undefined; }
console.log("found", find(f4, "b"), "after return:", f4.closed);
try { f4.readLine(); } catch (e) { console.log(`${e}`); }
console.log(f1, Object.keys(f1), JSON.stringify(f1), typeof f1.readLine, "closed" in f1);
