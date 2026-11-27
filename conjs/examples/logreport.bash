# The Bash version of logreport.con.js. Each step gets lines, so the fields are cut again with awk at each
# step, and the numbers of uniq -c are padded differently by each system (GNU and BSD), so awk prints them
# again.
log=access.log

echo "Requests by status:"
awk '{ print $4 }' "$log" | sort | uniq -c | awk '{ print $1, $2 }' | sort -k2

echo "Slow requests (over 100 ms):"
awk '{ ms = substr($5, 1, length($5) - 2); if (ms + 0 > 100) print ms "ms " $3 }' "$log" | sort -rn

count=$(grep -c ' 5[0-9][0-9] ' "$log")
echo "Server errors: $count"
grep ' 5[0-9][0-9] ' "$log" | while IFS= read -r line; do echo "  $line"; done
