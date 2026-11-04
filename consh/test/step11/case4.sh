# リダイレクト先にも適用する。2つ以上の語になればエラー
echo hello > g/a.t*
cat g/a.txt
echo bad > g/*.txt
