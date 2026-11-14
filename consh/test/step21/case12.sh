# 値が undefined の $(( )) はエラー（$? は 2。対話モードでなければ ConSh を終了ステータス 2 で終える。2026-11-14 にユーザーと決めた）。
# null は空の文字列。文の (( )) は、値が undefined でもエラーにしない（偽なので $? は 1）
echo "null: [$(( null ))]"
(( v = () => {} ))
(( v() ))
echo "statement: $?"
echo "value: $(( v() ))"
echo "not reached"
