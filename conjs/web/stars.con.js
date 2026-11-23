// 入力を待つ対話の見本「星ひろい」（docs/dev/PLATFORM-PLAN-ja.md の 4.6 節、段階 P5c）。stars.html が、wasm 版の VM で動かす。
// 矢印のキー（かクリック）で自分（四角）を 1 マスずつ動かし、星をすべて拾うと次の面に進む。おばけは、それぞれ一定の
// 間隔で自分に 1 マス近づき、ぶつかると終わる。Escape で、どの画面からでも終える。
//
// 画面の順（タイトル、遊ぶ所、結果）は、ふつうのループで書く。おばけは 1 匹ごとに spawn した処理で、sleep しながら
// 動く。描く処理は、1 コマごとに描いて host.wait("canvas.frame") で待つ。自分は nextEvent でキーを待つ。どの処理も
// game を共有するが、切り替わるのは待つ呼び出し（nextEvent、sleep、host.wait）の所だけなので、動かしている途中で
// ほかの処理が game を見ることはない。JavaScript で書くなら、画面ごとの状態の変数と keydown の受け手の中の分岐、
// おばけの setInterval と clearInterval になる（stars-events.js。compare.html で比べられる）
//
// このファイルはブラウザ用で、canvas に描く分だけを持つ。端末で遊ぶ版（stars-term.con.js。4.7 節、段階 P6a）は、
// このファイルから、画面の順とゲームの決まり（play）と盤の大きさを import し、描き方だけを自分で書く（2026-11-23 に
// 分けた。段階 P9 の import の見本、4.12 節）。canvas のない所で読み込まれたときは、export するだけで動き始めない。
export const COLS = 15, ROWS = 10;
const CELL = 32, TOP = 40;
const STAR = [[0, -1], [0.5878, -0.809], [0.9511, -0.309], [0.9511, 0.309], [0.5878, 0.809],
              [0, 1], [-0.5878, 0.809], [-0.9511, 0.309], [-0.9511, -0.309], [-0.5878, -0.809]];
let seed = Date.now() % 2147483646 + 1;   // 1 以上 2147483646 以下

if (host.available("canvas")) play(canvasUi());

// 遊ぶ。描き方は ui（canvasUi か、stars-term.con.js の端末の ui）が持つ：
//   click                   クリックで動けるか（タイトルの文に出す）
//   screen(lines)           タイトルと結果の画面（[文, 色] の並び）
//   paint(game, lv, first)  盤を描く（first は、面の最初のコマと最後の形）
//   frame()                 次のコマまで待つ
//   cleared(level)          面クリアを出す
//   end()                   おしまいの画面
export function play(ui) {
  try {
    while (true) {
      if (!title(ui)) break;
      const score = playGame(ui);
      if (score === undefined) break;
      if (!result(ui, score)) break;
    }
    ui.end();
    console.log("stars: bye");
  } catch (e) {
    // 標準入力がファイルやパイプで、尽きたとき（端末を閉じたときも）は、ふつうに終える
    if (e.message !== "nextEvent: the input has ended") throw e;
    console.log("stars: the input has ended");
  }
}

// 0 以上 n 未満の整数（Park と Miller の線形合同法。積は 2 の 46 乗より小さく、数の精度の中で正しく計算できる）
function rand(n) {
  seed = seed * 16807 % 2147483647;
  return Math.floor(seed / 2147483647 * n);
}

// 列に残った入力を捨てる（前の画面で押したキーを、次の画面に渡さない）
function flush() { while (pollEvent()) {} }

// キーかクリックを待つ。Escape なら偽
function waitStart() {
  flush();
  while (true) {
    const e = nextEvent();
    if (e.type === "click") return true;
    if (e.key === "Escape") return false;
    if (e.key !== "Shift" && e.key !== "Control" && e.key !== "Alt" && e.key !== "Meta") return true;
  }
}

function title(ui) {
  ui.screen([["星ひろい", "#f3a712"],
             ["矢印のキーで動いて、星をすべて拾う。", "#fafafa"],
             [ui.click ? "おばけにぶつかると終わり。クリックでも動ける。" : "おばけにぶつかると終わり。", "#fafafa"],
             [ui.click ? "キーかクリックで始める。Escape で終わる。" : "キーで始める。Escape で終わる。", "#a8c686"]]);
  return waitStart();
}

function result(ui, score) {
  ui.screen([["おばけにぶつかった", "#e4572e"],
             [`点：${score}`, "#fafafa"],
             [ui.click ? "キーかクリックでタイトルへ。Escape で終わる。" : "キーでタイトルへ。Escape で終わる。", "#a8c686"]]);
  sleep(300);   // 遊んでいた手が離れるまでのキーも捨てる
  return waitStart();
}

// 遊ぶ。終わったら点を返す（Escape なら undefined）
function playGame(ui) {
  const game = { score: 0, level: 0, over: false, quit: false };
  while (!game.over) {
    game.level++;
    runLevel(ui, game);
  }
  return game.quit ? undefined : game.score;
}

export function same(a, b) { return a.x === b.x && a.y === b.y; }

// 空いているマス（自分、星、おばけ、ほかの avoid のない所）
function freeCell(lv, avoid) {
  while (true) {
    const p = { x: rand(COLS), y: rand(ROWS) };
    const taken = [lv.player, ...lv.stars, ...lv.ghosts, ...avoid].some((q) => q && same(p, q));
    if (!taken) return p;
  }
}

function newLevel(game) {
  const lv = { player: { x: 0, y: 0 }, stars: [], ghosts: [], done: false, cleared: false };
  const nGhosts = Math.min(game.level + 1, 4);
  const interval = Math.max(560 - game.level * 60, 220);
  // おばけは、自分から離れた右下の側に置く
  while (lv.ghosts.length < nGhosts) {
    const g = { x: COLS - 1 - rand(5), y: ROWS - 1 - rand(4), interval: interval + rand(120) };
    if (!lv.ghosts.some((q) => same(q, g))) lv.ghosts.push(g);
  }
  for (let i = 0; i < 4 + game.level; i++) lv.stars.push(freeCell(lv, []));
  return lv;
}

function runLevel(ui, game) {
  const lv = newLevel(game);
  flush();
  const ghosts = lv.ghosts.map((g) => spawn(() => {
    while (true) {
      sleep(g.interval);
      if (lv.done) return;
      moveGhost(game, lv, g);
    }
  }));
  const painter = spawn(() => {
    let first = true;
    while (!lv.done) {
      ui.paint(game, lv, first);
      first = false;
      ui.frame();
    }
  });
  try {
    while (!lv.done) {
      const e = nextEvent(100);   // 100 ミリ秒ごとに起きて、おばけが立てた lv.done を見る
      if (!e) continue;
      if (e.type === "key" && e.key === "Escape") {
        game.quit = game.over = lv.done = true;
      } else if (e.type === "key") {
        const d = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[e.key];
        if (d) movePlayer(game, lv, d[0], d[1]);
      } else {
        movePlayer(game, lv, ...towards(lv.player, e.x / CELL - 0.5, (e.y - TOP) / CELL - 0.5));
      }
    }
  } finally {
    // 入力が尽きた（nextEvent の Error）ときも、おばけと描く処理を終えてから抜ける
    if (!lv.done) game.over = game.quit = lv.done = true;
    ghosts.forEach(join);
    join(painter);
  }
  ui.paint(game, lv, true);
  if (lv.cleared) {
    ui.cleared(game.level);
    sleep(800);
  }
}

// p から (x, y) への 1 マスの向き（離れているほうの軸で）
function towards(p, x, y) {
  const dx = x - p.x, dy = y - p.y;
  if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) return [0, 0];
  if (Math.abs(dx) >= Math.abs(dy)) return [Math.sign(dx), 0];
  return [0, Math.sign(dy)];
}

function inside(x, y) { return x >= 0 && x < COLS && y >= 0 && y < ROWS; }

function hit(game, lv) {
  if (lv.ghosts.some((g) => same(g, lv.player))) {
    game.over = lv.done = true;
  }
}

function movePlayer(game, lv, dx, dy) {
  const x = lv.player.x + dx, y = lv.player.y + dy;
  if (!inside(x, y)) return;
  lv.player = { x, y };
  const left = lv.stars.filter((s) => !same(s, lv.player));
  if (left.length < lv.stars.length) {
    lv.stars = left;
    game.score += 10;
    if (lv.stars.length === 0) lv.done = lv.cleared = true;
  }
  hit(game, lv);
}

// おばけを自分に 1 マス近づける（5 回に 1 回は、でたらめな向き）。ほかのおばけのいるマスには入らない
function moveGhost(game, lv, g) {
  const [dx, dy] = rand(5) === 0 ? [[1, 0], [-1, 0], [0, 1], [0, -1]][rand(4)] : towards(g, lv.player.x, lv.player.y);
  const p = { x: g.x + dx, y: g.y + dy };
  if (!inside(p.x, p.y) || lv.ghosts.some((q) => q !== g && same(q, p))) return;
  g.x = p.x;
  g.y = p.y;
  hit(game, lv);
}

// ── canvas に描く ──

function canvasUi() {
  host.call("canvas.font", "16px sans-serif");
  const draw = (cmds) => host.call("canvas.draw", cmds);
  return {
    click: true,
    screen: (lines) => {
      const cmds = [["fill_style", "#29335c"], ["fill_rect", 0, 0, COLS * CELL, TOP + ROWS * CELL]];
      lines.forEach(([text, color], i) => cmds.push(["fill_style", color], ["fill_text", text, 40, 110 + i * 32]));
      draw(cmds);
    },
    paint: (game, lv, first) => draw(paintCmds(game, lv)),
    frame: () => host.wait("canvas.frame"),
    cleared: (level) => draw([["fill_style", "#29335c"], ["fill_text", `${level} 面クリア`, 190, 26]]),
    end: () => draw([["fill_style", "#fafafa"], ["fill_rect", 0, 0, COLS * CELL, TOP + ROWS * CELL],
                     ["fill_style", "#333"], ["fill_text", "おしまい", 16, 26]]),
  };
}

function paintCmds(game, lv) {
  const W = COLS * CELL;
  const cmds = [["fill_style", "#fafafa"], ["fill_rect", 0, 0, W, TOP + ROWS * CELL],
                ["fill_style", "#333"],
                ["fill_text", `${game.level} 面　点 ${game.score}　残りの星 ${lv.stars.length}`, 16, 26],
                ["fill_style", "#eef0f4"], ["fill_rect", 0, TOP, W, ROWS * CELL],
                ["stroke_style", "#dde1e8"], ["line_width", 1], ["begin_path"]];
  for (let x = 0; x <= COLS; x++) cmds.push(["move_to", x * CELL, TOP], ["line_to", x * CELL, TOP + ROWS * CELL]);
  for (let y = 0; y <= ROWS; y++) cmds.push(["move_to", 0, TOP + y * CELL], ["line_to", W, TOP + y * CELL]);
  cmds.push(["stroke"]);
  for (const s of lv.stars) star(cmds, s.x * CELL + CELL / 2, TOP + s.y * CELL + CELL / 2);
  for (const g of lv.ghosts) ghost(cmds, g.x * CELL, TOP + g.y * CELL);
  const p = lv.player;
  cmds.push(["fill_style", game.over && !game.quit ? "#e4572e" : "#669bbc"],
            ["fill_rect", p.x * CELL + 5, TOP + p.y * CELL + 5, CELL - 10, CELL - 10]);
  return cmds;
}

// 中心 (cx, cy) の 5 つの角の星。STAR は、真上から 36 度ずつの向き（cos と sin。ConJS の Math にはないので、数で書く）
function star(cmds, cx, cy) {
  cmds.push(["fill_style", "#f3a712"], ["begin_path"]);
  STAR.forEach(([c, s], i) => {
    const r = i % 2 === 0 ? 12 : 5;
    cmds.push([i === 0 ? "move_to" : "line_to", cx + r * c, cy + r * s]);
  });
  cmds.push(["close_path"], ["fill"]);
}

// 左上 (x, y) のマスのおばけ
function ghost(cmds, x, y) {
  cmds.push(["fill_style", "#8e5572"], ["begin_path"], ["arc", x + 16, y + 14, 10, Math.PI, 0], ["fill"],
            ["fill_rect", x + 6, y + 14, 20, 12],
            ["fill_style", "#fff"], ["fill_rect", x + 10, y + 11, 4, 5], ["fill_rect", x + 18, y + 11, 4, 5]);
}
