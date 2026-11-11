# ジェネレータ（段階 B5）：別の評価で続けること、シェルの値の LazyList（読むときに next を呼ぶ）、エラー
(( nat = function* () { let n = 1; while (true) { console.log(`making ${n}`); yield n++ } } ))
(( g = nat() ))
echo "first: $(( g.next().value ))"
echo "second, in another evaluation: $(( g.next().value ))"
v=$(( [...(function* () { yield 1; yield* ["a", "b"] })()] ))
echo "spread: $v"
(( take = function* (it, k) { for (const x of it) { if (k-- <= 0) return; yield x } } ))
for x in $(( take(nat(), 3) )); do echo "shell got $x"; done
gs=$(( take(nat(), 2) ))
echo "all: $gs"
echo "typeof: $(( typeof g )), $(( `${g}` ))"
(( bad = function* () { yield 1; throw "oops" } ))
for x in $(( bad() )); do echo "bad gave $x"; done
echo "status $?"
(( yield 1 ))
echo "status $?"
