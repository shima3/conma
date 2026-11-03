echo $0 $1 $2 ${3-none} ${2} $# "$#"
echo pre$@suf "pre$@suf" "$@" $@
x=${@} y="$@" z="$1$2"
echo $? $! $$
