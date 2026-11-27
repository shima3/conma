// && と ||：最後に動いたものが失敗したときだけ投げる。2>$：標準エラーの行をキューに（段階 S3）
$ true && echo and-yes
$ false && echo never || echo or-yes
$ false || echo recovered
try {
  $ true && false
} catch (e) {
  console.log("thrown", e.status);
}
let errs;
$ sh -c "echo to-err 1>&2; echo to-out" 2>$ errs
console.log([...errs].join(","));
let o2;
let e2;
$ sh -c "echo a; echo b 1>&2" >$ o2 2>$ e2
console.log([...o2].join(","), [...e2].join(","));
// 動かないものの >$ は作らない：変数は前の値のまま
let skipped = "kept";
$ false && echo x >$ skipped || true
console.log(skipped);
// パイプと && を合わせる
let both;
$ printf "%s\n" 3 1 2 | sort -n >$ both && echo sorted
console.log([...both].join(","));
