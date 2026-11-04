# for 文のリダイレクトと return、break、continue
d=$1
function f (. xs) {
  for x in ${xs}; do
    if test $x = stop; then return 7; fi
    echo "f $x"
  done > $d/f.out
  echo not-here
}
f a b stop c
echo "f $?"
cat $d/f.out
for x in a b c; do
  for y in 1 2; do
    if test $y = 2; then continue x; fi
    echo "$x$y"; sh -c 'echo ext'
    if test $x = b; then break x; fi
  done > $d/$x.out
done
cat $d/a.out $d/b.out
