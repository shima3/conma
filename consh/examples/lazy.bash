# The Bash version of lazy.sh.
# for n in $(seq 1000000000) would first read all 10 GB of output, so read
# from a pipe instead. A pipe into while runs the loop in a subshell and
# loses the variable:
seq 1000000000 | while read -r n; do
  if [ $((n * n)) -gt 50 ]; then found=$n; break; fi
done
echo "after a pipe into while: '${found-}'"

# Process substitution keeps the loop in this shell:
while read -r n; do
  if [ $((n * n)) -gt 50 ]; then found=$n; break; fi
done < <(seq 1000000000)
echo "first n with n * n > 50: $found"

naturals() {
  local n=$1
  while true; do echo $n; n=$((n + 1)); done
}
while read -r n; do
  if [ $n -gt 3 ]; then break; fi
  echo "natural: $n"
done < <(naturals 1)
