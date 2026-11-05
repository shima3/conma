# A misspelled variable name. ConSh stops at the undefined variable instead of
# expanding it to an empty string, so the command never runs with "/*".
# Compare with typo.bash. ${x-default} gives a default explicitly.
cache_dir=${MYAPP_CACHE-/tmp/myapp-cache}
echo "cache: $cache_dir"
echo "would run: rm -rf $cahce_dir/*"
echo "not reached"
