// 「星ひろい」（stars.con.js）を、ふつうのブラウザの JavaScript で、イベント駆動で書いたもの（2026-11-23）。
// stars-events.html が動かす。compare.html で、ConJS 版と、async/await で書いたもの（stars-async.js）と並べて比べられる。
// 遊び方と決まりは ConJS 版と同じ：矢印のキー（かクリック）で自分（四角）を 1 マスずつ動かし、星をすべて拾うと次の面に
// 進む。おばけは、それぞれ一定の間隔で自分に 1 マス近づき、ぶつかると終わる。Escape で、どの画面からでも終える。
//
// JavaScript では、待つ所でプログラムを止められないので、次のように分けて書く。
// * 画面の順（タイトル、遊ぶ所、面クリアの間、結果）は、今の画面を表す変数 state と、キーとクリックの受け手 onInput の中の
//   分岐で書く。画面を移るのは、受け手と、時間が来たときの setTimeout。
// * おばけは 1 匹ごとに setInterval で動かし、面が終わったら clearInterval で止める。
// * 描くのは requestAnimationFrame の受け手で、面が終わったら cancelAnimationFrame で止める。
"use strict";
(() => {
  const COLS = 15, ROWS = 10, CELL = 32, TOP = 40;
  const STAR = [[0, -1], [0.5878, -0.809], [0.9511, -0.309], [0.9511, 0.309], [0.5878, 0.809],
                [0, 1], [-0.5878, 0.809], [-0.9511, 0.309], [-0.9511, -0.309], [-0.5878, -0.809]];
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");
  ctx.font = "16px sans-serif";
  let seed = Date.now() % 2147483646 + 1;   // 1 以上 2147483646 以下

  // 今の画面："title"、"play"、"clear"（面クリアの間）、"result"、"end"
  let state = "title";
  let game = null;      // { score, level, over }
  let lv = null;        // 今の面 { player, stars, ghosts, done, cleared }
  let ghostTimers = []; // おばけの setInterval
  let frameRequest = 0; // 描く requestAnimationFrame
  let ignoreUntil = 0;  // この時刻（performance.now()）より前のキーとクリックは捨てる（結果の画面）

  // 0 以上 n 未満の整数（Park と Miller の線形合同法）
  function rand(n) {
    seed = seed * 16807 % 2147483647;
    return Math.floor(seed / 2147483647 * n);
  }

  // キーとクリックを 1 つの形 { type: "key", key } か { type: "click", x, y }（canvas の座標）にして、onInput に渡す
  const QUIET_KEYS = new Set(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", " ", "Escape"]);
  canvas.addEventListener("keydown", (e) => {
    if (QUIET_KEYS.has(e.key)) e.preventDefault();
    onInput({ type: "key", key: e.key });
  });
  canvas.addEventListener("pointerdown", (e) => {
    canvas.focus();
    const r = canvas.getBoundingClientRect();
    onInput({ type: "click", x: (e.clientX - r.left) * canvas.width / r.width,
              y: (e.clientY - r.top) * canvas.height / r.height });
  });

  function onInput(e) {
    switch (state) {
      case "title":
      case "result":
        if (performance.now() < ignoreUntil) return;   // 遊んでいた手が離れるまでのキーは捨てる
        if (e.type === "key" && e.key === "Escape") { end(); return; }
        if (e.type === "key" && ["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;
        if (state === "title") startGame(); else showTitle();
        return;
      case "play":
        if (e.type === "key" && e.key === "Escape") {
          stopLevel();
          end();
        } else if (e.type === "key") {
          const d = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[e.key];
          if (d) movePlayer(d[0], d[1]);
        } else {
          movePlayer(...towards(lv.player, e.x / CELL - 0.5, (e.y - TOP) / CELL - 0.5));
        }
        return;
      default:   // 面クリアの間と、終わった後は、捨てる
        return;
    }
  }

  function screen(lines) {
    ctx.fillStyle = "#29335c";
    ctx.fillRect(0, 0, COLS * CELL, TOP + ROWS * CELL);
    lines.forEach(([text, color], i) => {
      ctx.fillStyle = color;
      ctx.fillText(text, 40, 110 + i * 32);
    });
  }

  function showTitle() {
    state = "title";
    screen([["星ひろい", "#f3a712"],
            ["矢印のキーで動いて、星をすべて拾う。", "#fafafa"],
            ["おばけにぶつかると終わり。クリックでも動ける。", "#fafafa"],
            ["キーかクリックで始める。Escape で終わる。", "#a8c686"]]);
  }

  function showResult() {
    state = "result";
    screen([["おばけにぶつかった", "#e4572e"],
            [`点：${game.score}`, "#fafafa"],
            ["キーかクリックでタイトルへ。Escape で終わる。", "#a8c686"]]);
    ignoreUntil = performance.now() + 300;
  }

  function end() {
    state = "end";
    ctx.fillStyle = "#fafafa";
    ctx.fillRect(0, 0, COLS * CELL, TOP + ROWS * CELL);
    ctx.fillStyle = "#333";
    ctx.fillText("おしまい", 16, 26);
  }

  function startGame() {
    game = { score: 0, level: 0, over: false };
    startLevel();
  }

  function same(a, b) { return a.x === b.x && a.y === b.y; }

  // 空いているマス（自分、星、おばけのない所）
  function freeCell() {
    while (true) {
      const p = { x: rand(COLS), y: rand(ROWS) };
      if (![lv.player, ...lv.stars, ...lv.ghosts].some((q) => same(p, q))) return p;
    }
  }

  function startLevel() {
    game.level++;
    lv = { player: { x: 0, y: 0 }, stars: [], ghosts: [], done: false, cleared: false };
    const nGhosts = Math.min(game.level + 1, 4);
    const interval = Math.max(560 - game.level * 60, 220);
    // おばけは、自分から離れた右下の側に置く
    while (lv.ghosts.length < nGhosts) {
      const g = { x: COLS - 1 - rand(5), y: ROWS - 1 - rand(4), interval: interval + rand(120) };
      if (!lv.ghosts.some((q) => same(q, g))) lv.ghosts.push(g);
    }
    for (let i = 0; i < 4 + game.level; i++) lv.stars.push(freeCell());
    state = "play";
    ghostTimers = lv.ghosts.map((g) => setInterval(() => moveGhost(g), g.interval));
    const frame = () => {
      paint();
      frameRequest = requestAnimationFrame(frame);
    };
    frame();
  }

  // おばけと描くのを止める
  function stopLevel() {
    lv.done = true;
    ghostTimers.forEach(clearInterval);
    ghostTimers = [];
    cancelAnimationFrame(frameRequest);
  }

  // 面が終わった（星をすべて拾ったか、おばけにぶつかった）：最後の形を描いて、次の画面へ
  function finishLevel() {
    stopLevel();
    paint();
    if (lv.cleared) {
      state = "clear";
      ctx.fillStyle = "#29335c";
      ctx.fillText(`${game.level} 面クリア`, 190, 26);
      // 最後の星と一緒におばけにぶつかったときは、クリアを見せてから結果へ（ConJS 版と同じ）
      setTimeout(() => (game.over ? showResult() : startLevel()), 800);
    } else {
      showResult();
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

  function hit() {
    if (lv.ghosts.some((g) => same(g, lv.player))) game.over = true;
  }

  function movePlayer(dx, dy) {
    const x = lv.player.x + dx, y = lv.player.y + dy;
    if (!inside(x, y)) return;
    lv.player = { x, y };
    const left = lv.stars.filter((s) => !same(s, lv.player));
    if (left.length < lv.stars.length) {
      lv.stars = left;
      game.score += 10;
      if (lv.stars.length === 0) lv.cleared = true;
    }
    hit();
    if (lv.cleared || game.over) finishLevel();
  }

  // おばけを自分に 1 マス近づける（5 回に 1 回は、でたらめな向き）。ほかのおばけのいるマスには入らない
  function moveGhost(g) {
    if (lv.done) return;
    const [dx, dy] = rand(5) === 0 ? [[1, 0], [-1, 0], [0, 1], [0, -1]][rand(4)] : towards(g, lv.player.x, lv.player.y);
    const p = { x: g.x + dx, y: g.y + dy };
    if (!inside(p.x, p.y) || lv.ghosts.some((q) => q !== g && same(q, p))) return;
    g.x = p.x;
    g.y = p.y;
    hit();
    if (game.over) finishLevel();
  }

  function paint() {
    const W = COLS * CELL;
    ctx.fillStyle = "#fafafa";
    ctx.fillRect(0, 0, W, TOP + ROWS * CELL);
    ctx.fillStyle = "#333";
    ctx.fillText(`${game.level} 面　点 ${game.score}　残りの星 ${lv.stars.length}`, 16, 26);
    ctx.fillStyle = "#eef0f4";
    ctx.fillRect(0, TOP, W, ROWS * CELL);
    ctx.strokeStyle = "#dde1e8";
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let x = 0; x <= COLS; x++) { ctx.moveTo(x * CELL, TOP); ctx.lineTo(x * CELL, TOP + ROWS * CELL); }
    for (let y = 0; y <= ROWS; y++) { ctx.moveTo(0, TOP + y * CELL); ctx.lineTo(W, TOP + y * CELL); }
    ctx.stroke();
    for (const s of lv.stars) star(s.x * CELL + CELL / 2, TOP + s.y * CELL + CELL / 2);
    for (const g of lv.ghosts) ghost(g.x * CELL, TOP + g.y * CELL);
    const p = lv.player;
    ctx.fillStyle = game.over ? "#e4572e" : "#669bbc";
    ctx.fillRect(p.x * CELL + 5, TOP + p.y * CELL + 5, CELL - 10, CELL - 10);
  }

  // 中心 (cx, cy) の 5 つの角の星（ConJS 版と同じ点）
  function star(cx, cy) {
    ctx.fillStyle = "#f3a712";
    ctx.beginPath();
    STAR.forEach(([c, s], i) => {
      const r = i % 2 === 0 ? 12 : 5;
      if (i === 0) ctx.moveTo(cx + r * c, cy + r * s); else ctx.lineTo(cx + r * c, cy + r * s);
    });
    ctx.closePath();
    ctx.fill();
  }

  // 左上 (x, y) のマスのおばけ
  function ghost(x, y) {
    ctx.fillStyle = "#8e5572";
    ctx.beginPath();
    ctx.arc(x + 16, y + 14, 10, Math.PI, 0);
    ctx.fill();
    ctx.fillRect(x + 6, y + 14, 20, 12);
    ctx.fillStyle = "#fff";
    ctx.fillRect(x + 10, y + 11, 4, 5);
    ctx.fillRect(x + 18, y + 11, 4, 5);
  }

  canvas.focus();
  showTitle();
})();
