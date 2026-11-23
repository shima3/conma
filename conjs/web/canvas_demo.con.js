// canvas の見本の ConJS 版（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節、段階 P3b）：canvas_demo.se と同じ、跳ねる玉。
// canvas.html（conjs/web）が、wasm 版の VM で動かす。待つ呼び出し（host.wait("canvas.frame")）を await なしで、ふつうの
// ループの中に書ける。もう 1 つの処理（spawn）が、30 コマごとに見出しの文字を変える。2 つの処理は、待つ所でだけ切り替わる
if (!host.available("canvas")) {
  console.log("canvas_demo.con.js: this platform has no canvas");
} else {
  const [w, h] = host.call("canvas.size");
  host.call("canvas.font", "16px sans-serif");

  // 0 以上 1 未満の数の列（線形合同法。canvas_demo.se と同じ）
  let seed = 42;
  function rand() {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    return seed / 2147483648;
  }
  const colors = ["#8e5572", "#a8c686", "#669bbc", "#f3a712", "#29335c", "#e4572e"];
  const balls = [];
  for (let i = 0; i < 12; i++) {
    const r = rand() * 16 + 8;
    balls.push({
      x: rand() * (w - 3 * r) + r, y: rand() * (h - 3 * r) + r,
      dx: rand() * 6 - 3, dy: rand() * 6 - 3, r, color: colors[i % colors.length],
    });
  }
  // p を d だけ動かし、r から size - r の外に出たら、向きを変える
  function bounce(p, d, r, size) {
    p += d;
    if (p < r) return [r + (r - p), -d];
    if (p > size - r) return [(size - r) - (p - (size - r)), -d];
    return [p, d];
  }

  const shared = { label: "ConJS" };
  spawn(() => {
    const words = ["ConJS", "spawn", "host.wait", "no await"];
    for (let i = 1; ; i++) {
      for (let k = 0; k < 30; k++) host.wait("canvas.frame");
      shared.label = words[i % words.length];
    }
  });

  for (let frame = 0; ; frame++) {
    const cmds = [["fill_style", "#fafafa"], ["fill_rect", 0, 0, w, h],
                  ["fill_style", "#333"], ["fill_text", `${shared.label} frame ${frame}`, 10, 24]];
    for (const b of balls) {
      [b.x, b.dx] = bounce(b.x, b.dx, b.r, w);
      [b.y, b.dy] = bounce(b.y, b.dy, b.r, h);
      cmds.push(["fill_style", b.color], ["begin_path"], ["arc", b.x, b.y, b.r, 0, 2 * Math.PI], ["fill"]);
    }
    host.call("canvas.draw", cmds);
    host.wait("canvas.frame");
  }
}
