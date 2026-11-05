# Summarize server.log. Lists are values: a function takes two lists as two
# arguments, and output goes straight into variables with > (first . rest).
# Compare with logsummary.bash.
errors=$(/usr/bin/grep ERROR server.log)
warnings=$(/usr/bin/grep WARN server.log)

function report (title lines) {
  /usr/bin/grep -c "" < (. lines) > (n)
  echo "$title ($n)"
  for line in ${lines}; do echo "  $line"; done
}

function summary (errors warnings) {
  report Errors ${errors}
  report Warnings ${warnings}
}

summary ${errors} ${warnings}

# The first error and the rest, without head, tail or read
/usr/bin/grep ERROR server.log > (first . rest)
echo "first error: $first"
OFS=" | " echo "later errors: $rest"
