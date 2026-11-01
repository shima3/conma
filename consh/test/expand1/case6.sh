HOGE=a; export HOGE=b; echo $HOGE
export NEW=1; echo $NEW
l=$(echo a; echo b); export l; echo $l "$l"
l=c; echo $l
P=x Q= cmd
