# Find the first n with n * n > 50 among 1 to 1,000,000,000.
# $(seq ...) is read lazily: seq stops once the loop breaks, and the variable
# set in the loop is still there afterwards. Compare with lazy.bash.
for n in $(/usr/bin/seq 1000000000); do
  square=$(/bin/expr $n \* $n)
  if [ $square -gt 50 ]; then found=$n; break; fi
done
echo "first n with n * n > 50: $found"

# A function can produce an endless list too
function naturals (n) {
  echo $n
  naturals $(/bin/expr $n + 1)
}
for n in $(naturals 1); do
  if [ $n -gt 3 ]; then break; fi
  echo "natural: $n"
done
