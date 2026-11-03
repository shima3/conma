list=${@}
words pre$@suf "pre$@suf" "$@" [$#] a$list "$list"
x=${@} y="$@"
words $x "$x" $y "$y"
