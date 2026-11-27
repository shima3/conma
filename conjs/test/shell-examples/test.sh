#!/bin/bash
# The examples of command lines in conjs/examples (logreport, todos, wordcounts; docs/dev/CONJS-SHELL-PLAN.md):
# each runs as ConJS (NAME.con.js) and as Bash (NAME.bash, with bash from PATH), in conjs/examples, and both
# print the same as NAME.log here, with the exit status; a difference is followed by the name of the run
# ("^ NAME.con.js" or "^ NAME.bash"; M1's review of shima3/conma#98). run_all.sh runs it.
here="$(cd "$(dirname "$0")"; pwd)"
examples="$here/../../examples"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/conjs-shell-examples.XXXXXX")" || exit 1
for c in logreport todos wordcounts
do
  (cd "$examples" && ../conjs $c.con.js < /dev/null > "$tmp/$c.log" 2>&1; echo "exit: $?" >> "$tmp/$c.log")
  diff "$here/$c.log" "$tmp/$c.log" || echo "^ $c.con.js"
  (cd "$examples" && bash $c.bash < /dev/null > "$tmp/$c.bash.log" 2>&1; echo "exit: $?" >> "$tmp/$c.bash.log")
  diff "$here/$c.log" "$tmp/$c.bash.log" || echo "^ $c.bash"
done
rm -rf "$tmp"
