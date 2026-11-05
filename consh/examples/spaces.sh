# Rename every .txt file in a directory to .md, where some names contain spaces.
# ConSh never splits a variable on spaces, so no quotes are needed.
# Compare with spaces.bash.
dir=$(/usr/bin/mktemp -d)
/usr/bin/touch "$dir/my report.txt" "$dir/notes 2026.txt" "$dir/todo.txt"

for f in ${-$dir/*.txt}; do
  /bin/ls $f > /dev/null
  name=$(/usr/bin/basename $f .txt)
  /bin/mv $f $dir/$name.md
  echo "renamed: $name"
done
/bin/ls $dir
/bin/rm -r $dir
