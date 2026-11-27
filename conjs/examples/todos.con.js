// Count the TODO lines of each note in notes/ with command lines (docs/dev/CONJS-SHELL-PLAN.md): a command
// that fails is taken by || or by catch, as in a shell, and the output of a command goes into a variable
// as a Queue. Compare with todos.bash.
let files;
$ find notes -name "*.txt" >$ files
const names = [...files].sort();
console.log(`${names.length} notes`);

let total = 0;
for (const f of names) {
  // grep -c fails (exit status 1) when no line matches: || takes it and goes on
  let n;
  $ grep -c TODO ${f} >$ n || echo "${f}: no TODO"
  const count = Number(n.text());
  if (count > 0) console.log(`${f}: ${count}`);
  total = total + count;
}
console.log(`total: ${total}`);

// A command that fails without || throws a CommandError, which catch takes
for (const f of names) {
  try {
    $ grep -q DONE ${f}
    console.log(`${f} has DONE`);
  } catch (e) {
    console.log(`${f} has no DONE (exit status ${e.status})`);
  }
}

// && runs the next command only when the one before it succeeds
$ test -d notes && echo "notes is a directory"
$ test -d nothing-here && echo "never printed" || echo "nothing-here is not a directory"
