// コマンド行（docs/dev/CONJS-SHELL-PLAN.md の段階 S1b）：外部コマンドと、<$、>$、>>$
$ echo hello world
const name = "a b";
$ echo "name: ${name}" 'single ${name}' x${name}y
let out;
$ printf "%s\n" one two three >$ out
for (const line of out) console.log(`[${line}]`);
const lines = ["c", "a", "b"];
let sorted;
$ sort <$ lines >$ sorted
console.log([...sorted].join(","));
let all;
$ echo first >$ all
$ echo second >>$ all
console.log([...all].join(","));
function* gen() {
  yield 3;
  yield 1;
  sleep(5);
  yield 2;
}
const g = gen();
let nums;
$ sort -n <$ g >$ nums
console.log([...nums].join(","));
try {
  $ false
} catch (e) {
  console.log(e.name, e.status, e.message);
}
try {
  $ no-such-command-here
} catch (e) {
  console.log(e.message);
}
$ printf "%s\n" \
  continued \
  lines
console.log("end");
