# The Bash version of spaces.sh. Every variable needs quotes.
dir=$(mktemp -d)
touch "$dir/my report.txt" "$dir/notes 2026.txt" "$dir/todo.txt"

# Without the quotes, Bash splits each name at the space:
for f in $dir/*.txt; do
  ls $f > /dev/null 2>&1 || echo "unquoted \$f breaks: $(basename "$f")"
done

for f in "$dir"/*.txt; do
  name=$(basename "$f" .txt)
  mv "$f" "$dir/$name.md"
  echo "renamed: $name"
done
ls "$dir"
rm -r "$dir"
