# 呼ぶときに確かめる Math のメンバーは、その関数とそれが呼ぶ数の関数が使うものだけ（REVIEW-ja.md の 2026-11-14（2）の 4）。
# 使わないメンバーを書き換えても wasm 版のまま正しく、使う（呼ぶ関数が使うものも含む）メンバーを書き換えると ConMa 版で動く
(( fl = (x) => Math.floor(x), useFl = (x) => fl(x) + 1, rd = (x) => Math.round(x), pi = () => Math.PI ))
echo "before: $(( fl(2.5) )) $(( useFl(2.5) )) $(( rd(2.5) )) $(( pi() ))"
(( Math.round = (x) => 7 ))
echo "round replaced: $(( fl(2.5) )) $(( useFl(2.5) )) $(( rd(2.5) )) $(( pi() ))"
(( Math.floor = (x) => 100 ))
echo "floor replaced: $(( fl(2.5) )) $(( useFl(2.5) )) $(( rd(2.5) )) $(( pi() ))"
(( Math.PI = 3 ))
echo "PI replaced: $(( pi() ))"
