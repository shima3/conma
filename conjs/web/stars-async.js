// 「星ひろい」（stars.con.js）を、ふつうのブラウザの JavaScript で、async/await で書いたもの（2026-11-23）。
// stars-async.html が動かす。compare.html で、ConJS 版と、イベント駆動で書いたもの（stars-events.js）と並べて比べられる。
// 遊び方と決まりは ConJS 版と同じ：矢印のキー（かクリック）で自分（四角）を 1 マスずつ動かし、星をすべて拾うと次の面に
// 進む。おばけは、それぞれ一定の間隔で自分に 1 マス近づき、ぶつかると終わる。Escape で、どの画面からでも終える。
//
// 形は ConJS 版と同じ（画面の順はふつうのループ、おばけは 1 匹ごとの処理、キーは nextEvent で待つ）だが、次のことが要る。
// * 待つものはどれも Promise なので、ConJS 版の nextEvent、pollEvent、sleep、host.wait("canvas.frame") にあたるもの
//   （キーとクリックの列と、それを待つ Promise。setTimeout と requestAnimationFrame の Promise）を、ここで作る。
// * 待つ呼び出しには await を付け、それを呼ぶ関数はみな async にし、その呼び出しにも await を付ける。付け忘れると、待たずに
//   Promise を受け取って先に進む（エラーにならない）。
// * おばけと描く処理は async 関数を呼んで始め、その Promise を Promise.all で待つ（ConJS 版の spawn と join）。
"use strict";
(() => {
  const COLS = 15, ROWS = 10, CELL = 32, TOP = 40;
  const STAR = [[0, -1], [0.5878, -0.809], [0.9511, -0.309], [0.9511, 0.309], [0.5878, 0.809],
                [0, 1], [-0.5878, 0.809], [-0.9511, 0.309], [-0.9511, -0.309], [-0.5878, -0.809]];
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");
  ctx.font = "16px sans-serif";
  let seed = Date.now() % 2147483646 + 1;   // 1 以上 2147483646 以下

  // キーとクリックの列。{ type: "key", key } か { type: "click", x, y }（canvas の座標）
  const events = [];
  let waiter = null;   // 待っている nextEvent の resolve
  function put(e) {
    if (waiter) {
      const w = waiter;
      waiter = null;
      w(e);
    } else {
      events.push(e);
    }
  }
  const QUIET_KEYS = new Set(["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", " ", "Escape"]);
  canvas.addEventListener("keydown", (e) => {
    if (QUIET_KEYS.has(e.key)) e.preventDefault();
    put({ type: "key", key: e.key });
  });
  canvas.addEventListener("pointerdown", (e) => {
    canvas.focus();
    const r = canvas.getBoundingClientRect();
    put({ type: "click", x: (e.clientX - r.left) * canvas.width / r.width,
          y: (e.clientY - r.top) * canvas.height / r.height });
  });

  // 次のキーかクリック（の Promise）。ms を渡すと、ms ミリ秒のうちに来なければ null
  function nextEvent(ms) {
    if (events.length > 0) return Promise.resolve(events.shift());
    return new Promise((resolve) => {
      let timer;
      waiter = (e) => {
        clearTimeout(timer);
        resolve(e);
      };
      if (ms !== undefined) {
        timer = setTimeout(() => {
          waiter = null;
          resolve(null);
        }, ms);
      }
    });
  }
  function pollEvent() { return events.length > 0 ? events.shift() : null; }
  function sleep(ms) { return new Promise((resolve) => setTimeout(resolve, ms)); }
  function frame() { return new Promise((resolve) => requestAnimationFrame(resolve)); }

  main();

  async function main() {
    while (true) {
      if (!await title()) break;
      const score = await play();
      if (score === undefined) break;
      if (!await result(score)) break;
    }
    ctx.fillStyle = "#fafafa";
    ctx.fillRect(0, 0, COLS * CELL, TOP + ROWS * CELL);
    ctx.fillStyle = "#333";
    ctx.fillText("おしまい", 16, 26);
  }

  // 0 以上 n 未満の整数（Park と Miller の線形合同法）
  function rand(n) {
    seed = seed * 16807 % 2147483647;
    return Math.floor(seed / 2147483647 * n);
  }

  // 列に残った入力を捨てる（前の画面で押したキーを、次の画面に渡さない）
  function flush() { while (pollEvent()) {} }

  // キーかクリックを待つ。Escape なら偽
  async function waitStart() {
    flush();
    while (true) {
      const e = await nextEvent();
      if (e.type === "click") return true;
      if (e.key === "Escape") return false;
      if (e.key !== "Shift" && e.key !== "Control" && e.key !== "Alt" && e.key !== "Meta") return true;
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

  async function title() {
    screen([["星ひろい", "#f3a712"],
            ["矢印のキーで動いて、星をすべて拾う。", "#fafafa"],
            ["おばけにぶつかると終わり。クリックでも動ける。", "#fafafa"],
            ["キーかクリックで始める。Escape で終わる。", "#a8c686"]]);
    return await waitStart();
  }

  async function result(score) {
    screen([["おばけにぶつかった", "#e4572e"],
            [`点：${score}`, "#fafafa"],
            ["キーかクリックでタイトルへ。Escape で終わる。", "#a8c686"]]);
    await sleep(300);   // 遊んでいた手が離れるまでのキーも捨てる
    return await waitStart();
  }

  // 遊ぶ。終わったら点を返す（Escape なら undefined）
  async function play() {
    const game = { score: 0, level: 0, over: false, quit: false };
    while (!game.over) {
      game.level++;
      await runLevel(game);
    }
    return game.quit ? undefined : game.score;
  }

  function same(a, b) { return a.x === b.x && a.y === b.y; }

  // 空いているマス（自分、星、おばけのない所）
  function freeCell(lv) {
    while (true) {
      const p = { x: rand(COLS), y: rand(ROWS) };
      if (![lv.player, ...lv.stars, ...lv.ghosts].some((q) => same(p, q))) return p;
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
    for (let i = 0; i < 4 + game.level; i++) lv.stars.push(freeCell(lv));
    return lv;
  }

  async function runLevel(game) {
    const lv = newLevel(game);
    flush();
    const ghosts = lv.ghosts.map(async (g) => {
      while (true) {
        await sleep(g.interval);
        if (lv.done) return;
        moveGhost(game, lv, g);
      }
    });
    const painter = (async () => {
      while (!lv.done) {
        paint(game, lv);
        await frame();
      }
    })();
    while (!lv.done) {
      const e = await nextEvent(100);   // 100 ミリ秒ごとに起きて、おばけが立てた lv.done を見る
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
    await Promise.all([...ghosts, painter]);
    paint(game, lv);
    if (lv.cleared) {
      ctx.fillStyle = "#29335c";
      ctx.fillText(`${game.level} 面クリア`, 190, 26);
      await sleep(800);
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

  function paint(game, lv) {
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
    ctx.fillStyle = game.over && !game.quit ? "#e4572e" : "#669bbc";
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
})();
