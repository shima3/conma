# パイプラインの中の return はエラー
function f () { if true; then return 3; fi | cat; }
f
