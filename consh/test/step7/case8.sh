# & の中の return はエラー
function f () { if true; then return 1; fi & wait; }
f
