// 終わりのないジェネレータも、読むのをやめるコマンドに渡せる：移す処理は、キューが 64 になると待ち、コマンドが終わるとやめる（M1 のレビュー、shima3/conma#96）
function* nat() { let i = 0; for (;;) { yield i; i = i + 1; } }
const g = nat();
$ head -3 <$ g
console.log("done");
