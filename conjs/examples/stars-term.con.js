// 「星ひろい」を端末で遊ぶ版（docs/dev/PLATFORM-PLAN-ja.md の 4.7 節、段階 P6a）：ANSI のエスケープの並びで端末に描き、
// キーを標準入力から読む。
//   conjs/conjs conjs/examples/stars-term.con.js
// ゲームの決まり（面の作り方、おばけ、キーで動くこと）は、ブラウザ用の conjs/web/stars.con.js から import し、画面の順と
// 描き方はここに書く（段階 P9 の import の見本、4.12 節。2026-11-25 から。それまでは画面の順も import した）。自分は @、
// 星は *、おばけは G。ブラウザと違い、標準入力がファイルやパイプなら入力が尽きるので、その Error を受け止めて終える。
import { COLS, ROWS, flush, waitStart, same, newLevel, startGhosts, onKey } from "../web/stars.con.js";

const ESC = "\u001b";

try {
  while (true) {
    if (!title()) break;
    const score = playGame();
    if (score === undefined) break;
    if (!result(score)) break;
  }
  console.log("stars: bye");
} catch (e) {
  // 標準入力がファイルやパイプで、尽きたとき（端末を閉じたときも）は、ふつうに終える
  if (e.message !== "nextEvent: the input has ended") throw e;
  console.log("stars: the input has ended");
}

function title() {
  term(["星ひろい",
        "矢印のキーで動いて、星をすべて拾う。",
        "おばけにぶつかると終わり。",
        "キーで始める。Escape で終わる。"]);
  return waitStart();
}

function result(score) {
  term(["おばけにぶつかった",
        `点：${score}`,
        "キーでタイトルへ。Escape で終わる。"]);
  sleep(300);   // 遊んでいた手が離れるまでのキーも捨てる
  return waitStart();
}

// 遊ぶ。終わったら点を返す（Escape なら undefined）
function playGame() {
  const game = { score: 0, level: 0, over: false, quit: false };
  while (!game.over) {
    game.level++;
    runLevel(game);
  }
  return game.quit ? undefined : game.score;
}

function runLevel(game) {
  const lv = newLevel(game);
  flush();
  const ghosts = startGhosts(game, lv);
  const painter = spawn(() => {
    let last = "";   // 前に描いた盤（同じなら描かない）
    while (!lv.done) {
      const text = board(game, lv);
      if (text !== last) term([text]);
      last = text;
      sleep(100);
    }
  });
  try {
    while (!lv.done) {
      const e = nextEvent(100);   // 100 ミリ秒ごとに起きて、おばけが立てた lv.done を見る
      if (e) onKey(game, lv, e.key);
    }
  } finally {
    // 入力が尽きた（nextEvent の Error）ときも、おばけと描く処理を終えてから抜ける
    if (!lv.done) game.over = game.quit = lv.done = true;
    ghosts.forEach(join);
    join(painter);
  }
  term([board(game, lv)]);
  if (lv.cleared) {
    console.log(`${game.level} 面クリア`);
    sleep(800);
  }
}

// 端末に、画面を消してから行を出す
function term(lines) { console.log(`${ESC}[2J${ESC}[H${lines.join("\n")}`); }

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
