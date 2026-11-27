# The Bash version of wordcounts.con.js. A command run with & cannot give a value back to the script, so each
# writes its result to a file of its own in a temporary directory, which the script reads after wait.
files=$(find notes -name '*.txt')
tmp=$(mktemp -d)
for f in $files; do
  (n=$(wc -w < "$f" | tr -d ' '); echo "$f $n" > "$tmp/$(basename "$f")") &
done
wait
sort "$tmp"/*
total=0
for r in "$tmp"/*; do
  n=$(cut -d ' ' -f 2 < "$r")
  total=$((total + n))
done
rm -r "$tmp"
echo "total $total"
