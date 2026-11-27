// Count the words of each note in notes/ at the same time (docs/dev/CONJS-SHELL-PLAN.md): each command runs
// in a task of spawn, as & does in a shell, and the results come back through one Queue, which every task
// writes. Compare with wordcounts.bash, which needs a directory of temporary files for the results.
let files;
$ find notes -name "*.txt" >$ files
const names = [...files];

const results = new Queue();
const tasks = names.map((f) => spawn(() => {
  let out;
  $ wc -w ${f} >$ out
  // wc writes the count first, padded with spaces on some systems
  const count = Number([...out][0].trim().split(" ")[0]);
  results.push(`${f} ${count}`);
  return count;
}));
let total = 0;
for (const t of tasks) total = total + join(t);
results.close();

// The tasks end in any order: sort the lines
$ sort <$ results
console.log(`total ${total}`);
