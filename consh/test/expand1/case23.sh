# for、break、continue
l=(a "" b) s=""
for x in $l ${l} "$l" ${s} $s x${l}; do cmd $x; done
echo "after: $x"
for x in; do cmd never; done
echo "still: $x"
for i in 1 2 3; do
  for j in p q; do
    if_none=1
    cmd $i $j
    continue i
  done
done
for i in 1 2; do for i in a b; do cmd $i; break; done; cmd outer $i; done
for i in 1 2; do y=$i; done; cmd y=$y
n=$(for v in a b; do echo $v; done )
cmd ${n}
for x in ${undefined-(u v)}; do cmd $x; break x; done
