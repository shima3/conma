js '1 + 2'
js '["a", "b"]' > (p q)
echo "p=$p q=$q"
# 時間を待つ JavaScript（jsc のイベントループで動かす）
js 'new Promise(r => setTimeout(() => r("later"), 10))'
echo after
