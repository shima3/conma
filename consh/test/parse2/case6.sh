# for、break、continue
for x in a $l "b c"; do echo $x; done
for x in   # comment
do
  for y in; do break x; continue; done
done > out | cat
for i in a b
# comment

do break; done
echo for in do done
