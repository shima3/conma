// 分割代入で Math のメンバーを書き換える（REVIEW-ja.md の 2026-11-14（2）の 3）。Math を使う関数は数の関数にならず、
// JavaScript と同じく、書き換えた関数を呼ぶ。Math を使わない関数は、数の関数のまま
function fl(x) { return Math.floor(x) }
function rd(x) { return Math.round(x) }
function add(a, b) { return a + b }
console.log(fl(1.5), rd(1.5), add(1, 2))
;[Math.floor] = [(x) => 42]
;({ round: Math.round } = { round: (x) => -1 })
console.log(fl(1.5), rd(1.5), add(1, 2))
