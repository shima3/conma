# 配列を使う数の関数（consh/SJS-WASM-ARRAY-PLAN-ja.md の段階 A4）：(( )) の「名前 = 関数式」で配列を読み書きする関数も、wasm 版の
# VM では WebAssembly で動き、引数の配列（SJS変数のものも）を写して、書き換えた数を書き戻す。ほかの VM では ConMa 版で動き、同じログになる
(( sum = (a) => { let s = 0; for (const v of a) s += v; return s } ))
(( xs = [1, 2, 3] ))
echo "sum: $(( sum(xs) ))"
(( scale = (a, k) => { for (let i = 0; i < a.length; i++) a[i] *= k } ))
(( scale(xs, 10) ))
echo "xs: $(( JSON.stringify(xs) ))"
# 同じ配列を 2 回入れた組：u と v は同じ配列
(( bump = (ps) => { for (const [u, v] of ps) { u[0] += 1; v[0] += 1 } } ))
(( s = [5, 6] ))
(( bump([[s, s]]) ))
echo "s: $(( JSON.stringify(s) ))"
# 配列を渡す数の関数どうしの呼び出し
(( total = (m) => { let t = 0; for (const row of m) t += sum(row); return t } ))
echo "total: $(( total([[1, 2], [3]]) ))"
# 数でない要素は写さない（ConMa 版で動く）
echo "length: $(( (n => n)(0) + ((a) => a.length)(["x", "y"]) ))"
(( big = [] ))
(( fill = (n) => { for (let i = 0; i < n; i++) big.push(i % 7) } ))
(( fill(1000) ))
echo "big sum: $(( sum(big) ))"
# 呼ぶ数の関数の名前を書き換えると、ConMa 版で動き、新しい関数を呼ぶ
(( sum = (a) => -1 ))
echo "total after sum changed: $(( total([[1, 2], [3]]) ))"
