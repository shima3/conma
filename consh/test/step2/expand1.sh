list=${@} e= s="" x=5
words a$list $list.txt "$list.txt" $list$list x$@ "x$@" "$@" $@
words $e a$e "$e" $s a$s "$s" "" '' ${undefined_v-} a${undefined_v-d} "${undefined_v-(u v)}"
words $0 $1 $2 ${3-none} ${2} $# "$#" pre$@suf "pre$@suf"
y=$x z=a$x w=a${list} v=${list}${list}
words $y $z $w $v "$w"
x=1 y=2 words $x $y
HOGE=a; export HOGE=b; words $HOGE
export NEW_V=1; words $NEW_V
export l=${list}; words $l "$l"
words 'it''s' "a"'b'c\ d "\$ \" \\ \a" $ $. ${1}0 $10
words a\
b "c\
d" 'e\
f'
