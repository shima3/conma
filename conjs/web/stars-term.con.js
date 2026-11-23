// 「星ひろい」を端末で遊ぶ版（docs/dev/PLATFORM-PLAN-ja.md の 4.7 節、段階 P6a）：ANSI のエスケープの並びで端末に描き、
// キーを標準入力から読む。
//   conjs/conjs conjs/web/stars-term.con.js
// 画面の順とゲームの決まり（play）は、ブラウザ用の stars.con.js から import し、ここには描き方だけを書く（2026-11-23 に
// 分けた。段階 P9 の import の見本、4.12 節）。自分は @、星は *、おばけは G。
import { play, same, COLS, ROWS } from "./stars.con.js";

const ESC = "\u001b";

// 端末に、画面を消してから行を出す
function term(lines) { console.log(`${ESC}[2J${ESC}[H${lines.join("\n")}`); }

let last = "";   // 前に描いた盤（同じなら描かない）

play({
  click: false,
  screen: (lines) => term(lines.map(([text, color]) => text)),
  paint: (game, lv, first) => {
    const text = board(game, lv);
    if (first || text !== last) term([text]);
    last = text;
  },
  frame: () => sleep(100),
  cleared: (level) => console.log(`${level} 面クリア`),
  end: () => {},
});

// 盤の文字列（色は、ANSI のエスケープの並び）
function board(game, lv) {
  const color = (code, ch) => `${ESC}[${code}m${ch}${ESC}[0m`;
  const lines = [`${game.level} 面　点 ${game.score}　残りの星 ${lv.stars.length}`];
  for (let y = 0; y < ROWS; y++) {
    const row = [];
    for (let x = 0; x < COLS; x++) {
      const p = { x, y };
      if (same(lv.player, p)) row.push(color(game.over && !game.quit ? 31 : 36, "@"));
      else if (lv.ghosts.some((g) => same(g, p))) row.push(color(35, "G"));
      else if (lv.stars.some((s) => same(s, p))) row.push(color(33, "*"));
      else row.push(".");
    }
    lines.push(row.join(" "));
  }
  return lines.join("\n");
}
