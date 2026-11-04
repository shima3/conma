# command は ${s} の空文字列を消さずに渡す。$s は消え、"$s" は空文字列を渡す
s=""
command printf "[%s]" x ${s} y; echo
printf "[%s]" x ${s} y; echo
command printf "[%s]" x $s y; echo
printf "[%s]" x $s y; echo
command printf "[%s]" x "$s" y; echo
printf "[%s]" x "$s" y; echo
