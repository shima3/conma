function g () { return 3 }
echo "$(g; echo $?)"
