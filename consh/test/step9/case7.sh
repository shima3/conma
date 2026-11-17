# 正しくない値を export すると、覚えた結果があっても、次のコマンドでエラーになる（組み込みコマンドでも。前と同じ）
echo before
echo again
export ARG_MAX=-1
echo not reached
