# エラー：打ち間違えたコマンドは、今までどおり見つからない（$? は 127）
nosuchcommand a b
echo "status $?"
nosuch | wc -l | tr -d ' '
undefinedName + 1
echo "status $?"
# JS の例外は js: と同じく $? は 1
null.x
echo "status $?"
Promise.reject(new Error("rejected"))
echo "status $?"
# JS としてもシェルとしても読めなければ、JS の構文エラー（$? は 1）
z = )
echo "status $?"
