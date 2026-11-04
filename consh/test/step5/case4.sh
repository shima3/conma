# 2>&1 で標準エラー出力もパイプに入る。2>&1 > file なら標準エラー出力だけが入る
sh -c 'echo out; echo err >&2' 2>&1 | sort
sh -c 'echo out; echo err >&2' 2>&1 > /dev/null | sed -e 's/^/got /'
# パイプの要素の中の $( ) は、その要素の標準入力を読む
echo piped | echo $(cat)
