# async と Promise（段階 B5b）：評価ごとに、マイクロタスクの列を最後まで動かす
(( double = async (x) => { await null; console.log(`doubling ${x}`); return x * 2 } ))
(( p = double(21) ))
echo "status $? after the evaluation"
echo "value: $(( p ))"
v=$(( double(5) ))
echo "v=$v, typeof $(( typeof p )), $(( `${p}` ))"
echo "then: $(( p.then((x) => x + 1) ))"
echo "all: $(( Promise.all([double(1), double(2)]) ))"
(( Promise.reject("refused") ))
echo "status $?"
(( q = Promise.reject("handled later") ))
echo "status $?"
(( r = Promise.reject("caught").catch((e) => `got ${e}`) ))
echo "r: $(( r ))"
f=$(( async (a) => { await a; return Number(a) + 1 } ))
$f 41 2> (c)
echo "c=$c status $?"
(( await 1 ))
echo "status $?"
