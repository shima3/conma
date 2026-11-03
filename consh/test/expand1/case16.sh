# ${x} と $x、既定値、値 (語 ...)、OFS
l=(a "" b) e=()
echo $l ${l} "$l" x${l}y "${l}" $e ${e} "$e"
y=${l} z=x${l}y w=$e v=${e} u="$l"
echo ${undefined-d} ${undefined-} "${undefined-"a b"}" ${undefined-(1 2)} ${l-z} ${1-none}
d=${undefined-(p q)} c=${undefined-${l}}
export OFS=,
echo "$l" "${undefined-(1 2)}"
OFS=- s="$l"
export t=${l}
echo $'tab\there' $s
