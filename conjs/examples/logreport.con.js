// Summarize access.log with command lines (docs/dev/CONJS-SHELL-PLAN.md): external commands and ConJS
// functions in one pipeline. Between functions, the records stay objects; only what goes to an external
// command becomes lines. Compare with logreport.bash, which has to cut the fields again at each step.
const log = "access.log";

// The records of the lines: { time, method, path, status, ms }
function fields(args, input, output) {
  for (const line of input) {
    const f = line.split(" ");
    output.push({ time: f[0], method: f[1], path: f[2], status: f[3], ms: Number(f[4].slice(0, -2)) });
  }
}

// How many records have each value of the field args[0], as "count value" lines
function countBy(args, input, output) {
  const counts = {};
  const order = [];
  for (const r of input) {
    const k = r[args[0]];
    if (counts[k] === undefined) {
      counts[k] = 0;
      order.push(k);
    }
    counts[k] = counts[k] + 1;
  }
  for (const k of order) output.push(`${counts[k]} ${k}`);
}

// The records slower than args[0] milliseconds
function slow(args, input, output) {
  const limit = Number(args[0]);
  for (const r of input) if (r.ms > limit) output.push(`${r.ms}ms ${r.path}`);
}

console.log("Requests by status:");
const lines = fs.readLines(log);
$ $fields <$ lines | $countBy status | sort -k2

console.log("Slow requests (over 100 ms):");
$ cat ${log} | $fields | $slow 100 | sort -rn

let errors;
$ grep " 5[0-9][0-9] " ${log} >$ errors
const found = [...errors];
console.log(`Server errors: ${found.length}`);
for (const line of found) console.log(`  ${line}`);
