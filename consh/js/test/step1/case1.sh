# 結果の出し方：値、配列、入れ子の配列、undefined、オブジェクト、数値と真偽値
js '1 + 2'
js '"text with spaces"'
js '["a", "b", ["c", "d"]]' > (p q r)
echo "p=$p q=$q"
for e in $r; do echo "r: $e"; done
js '[1, null, undefined, true]' > (a b c d)
echo "a=$a b=[$b] c=[$c] d=$d"
js 'undefined'
js 'null'
js '({a: 1, b: [true, "x"]})'
js '10n ** 20n'
js '"日本語\tと\nタブと改行"'
# 「識別子 =」でなければコード（代入式）
js 'jsGlobal = 5'
js 'jsGlobal * 2'
