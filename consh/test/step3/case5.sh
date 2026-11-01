echo plain echo
cd /
pwd
cd /no/such/dir
echo "status $?"
function echo (s) { builtin echo "my echo: $s" }
echo hi
builtin echo builtin
command echo external
