# シェルの標準入力も stdin で読める。その後の外部コマンドは続きから読む
stdin > (top)
echo "top=$top"
cat
