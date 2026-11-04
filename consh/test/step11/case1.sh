# *、?、[...]（範囲、否定、先頭の ]）。. で始まる名前は、パターンが . で始まるときだけ
echo g/*.txt
echo g/?.txt g/??
echo g/[ab].txt g/[a-c]* g/[!a-c].txt g/x[]1].txt
echo g/.* g/*.h*
# 一致しなければ語を残す。引用符の中と \ の後ろは記号にならない
echo g/*.none "g/*.txt" g/\*.txt g/'x[1]'.txt
# 2つのディレクトリにまたがれば、全体を文字コード順に並べる（d-2 が d1 より前）
echo g/*/x
# 最後の / はディレクトリだけ
echo g/d*/
# 絶対パス
d=$(pwd)

ls $d/g/c.* | sed "s|$d|PWD|"
