x=1
echo $(x=2; echo $x) $x
echo $(echo "a b"; echo) "$(echo a; echo b)"
echo $(echo $(echo inner))
