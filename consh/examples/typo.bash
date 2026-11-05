# The Bash version of typo.sh. The misspelled $cahce_dir is empty, so the
# command becomes rm -rf /*. (set -u would catch it, but it is off by default.)
cache_dir=${MYAPP_CACHE-/tmp/myapp-cache}
echo "cache: $cache_dir"
echo "would run: rm -rf $cahce_dir/*"
echo "not reached"
