# The Bash version of logsummary.sh. A function cannot take two arrays, so the
# lines are kept in strings and split again with read; an empty string must be
# checked, or it counts as one line.
errors=$(grep ERROR server.log)
warnings=$(grep WARN server.log)

report() {
  local title=$1 lines=$2 n=0 line
  if [ -n "$lines" ]; then n=$(printf '%s\n' "$lines" | grep -c ""); fi
  echo "$title ($n)"
  if [ -n "$lines" ]; then
    while IFS= read -r line; do echo "  $line"; done <<< "$lines"
  fi
}

summary() {
  report Errors "$1"
  report Warnings "$2"
}

summary "$errors" "$warnings"

first=$(grep ERROR server.log | head -1)
rest=()
while IFS= read -r line; do rest+=("$line"); done < <(grep ERROR server.log | tail -n +2)
echo "first error: $first"
joined=$(printf ' | %s' "${rest[@]}")
echo "later errors: ${joined# | }"
