# for、break、continue
l=(a "" b) s=""
for x in $l ${l} "$l" ${s} $s x${l}; do echo $x; done
echo "after: $x"
for x in; do echo never; done
echo "still: $x"
for i in 1 2 3; do
  for j in p q; do
    if_none=1
    echo $i $j
    continue i
  done
done
for i in 1 2; do for i in a b; do echo $i; break; done; echo outer $i; done
for i in 1 2; do y=$i; done; echo y=$y
for x in ${undefined-(u v)}; do echo $x; break x; done
