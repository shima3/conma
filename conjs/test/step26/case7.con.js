// 書き始めるのが遅い前のコマンド：次のコマンドは、空のパイプを読んで待つ（Windows では、shima3/conma#99 の後に、cat が NUL を読んだ）
for (let i = 0; i < 3; i++) {
  let a, b;
  $ sh -c "sleep 0.5; printf out" | cat >$ [a]
  $ sh -c "sleep 0.5; printf out" | wc -c >$ [b]
  console.log(JSON.stringify(a), b.trim());
}
// 遅い読み手：前のコマンドは、パイプに空きができるまで待ち、読み手が終われば止まる
let first;
$ yes | sh -c "sleep 1; head -1" >$ [first]
console.log(first);
// 間に cat をはさんでも、読むのをやめれば前のコマンドは止まる
$ yes | cat | head -2
