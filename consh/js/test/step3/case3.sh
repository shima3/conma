# 変数に入れるときは、要素を読んだときに作る遅延の LazyList
js nat= '(function* () { let i = 0; while (true) { console.log("make " + i); yield i++ } })()'
echo "made the list"
for e in ${nat}; do echo "e=$e"; break; done
# 作った要素は覚えておき、2回目は作らない
for e in ${nat}; do echo "again e=$e"; break; done
js three= '(function* () { yield "x"; yield "y"; yield "z" })()'
for e in ${three}; do echo "three: $e"; done
echo "three as a string: $three"
# 途中の例外は、メッセージを出して LazyList を終える（$? は変えない）
js bad= '(function* () { yield 1; throw new Error("lazy error") })()'
for e in ${bad}; do echo "bad: $e"; done
echo "status $?"
