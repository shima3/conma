list=$(echo a; echo b) list2=$(echo 1; echo 2)
echo a$list $list.txt "$list.txt" $list$list2
echo $@$list "$list" x$@
