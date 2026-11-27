# The Bash version of todos.con.js. The output of grep -c and its exit status come apart: the status is
# what the if sees, the count is what $( ) gives.
files=$(find notes -name '*.txt' | sort)
count=0
for f in $files; do count=$((count + 1)); done
echo "$count notes"

total=0
for f in $files; do
  if n=$(grep -c TODO "$f"); then
    echo "$f: $n"
  else
    echo "$f: no TODO"
  fi
  total=$((total + n))
done
echo "total: $total"

for f in $files; do
  grep -q DONE "$f"
  status=$?
  if [ $status -eq 0 ]; then
    echo "$f has DONE"
  else
    echo "$f has no DONE (exit status $status)"
  fi
done

test -d notes && echo "notes is a directory"
test -d nothing-here && echo "never printed" || echo "nothing-here is not a directory"
