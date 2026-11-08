# 本体の中の例外は、$? を 1 にする
js 'const x = 1; throw new Error("from a body")'
echo "status $?"
js 'await Promise.reject(new TypeError("rejected")); return 1'
echo "status $?"
# 本体の結果が関数ならエラー
js 'return () => 1'
echo "status $?"
