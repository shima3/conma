# > (a) 2> (b)：標準出力と標準エラー出力を別々の VProc が読む。両方の変数リストが一杯になれば、止まらないコマンドも止める
sh -c 'echo e >&2; exec yes o' > (a) 2> (b)
echo "a=$a b=$b status=$?"
