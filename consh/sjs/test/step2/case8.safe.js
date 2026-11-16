// 文字列の charAt と添字の境目（2026-11-16）：charAt は添字を整数にし（undefined と NaN は 0、0 の方へ切り捨て）、
// 負の数と長さ以上は ""。s[i] は整数の添字だけが文字で、ほかは undefined
const s = "héllo";
console.log([s.charAt(0), s.charAt(1), s.charAt(4), s.charAt(5), s.charAt(-1), s.charAt(-5)]);
console.log([s.charAt(), s.charAt(1.9), s.charAt(-0.5), s.charAt(NaN), s.charAt("2"), s.charAt(Infinity), s.charAt(-Infinity), s.charAt(null), s.charAt(true)]);
console.log(["".charAt(0), "".charAt(-1), "x".charAt(0)]);
console.log([s[0], s[1], s[4], s[5], s[-1], s[1.5], s["1"], s["01"], s.length, s["length"], ""[0], "".length]);
let n = 0;
for (let i = 0; i < s.length; i++) { if (s[i] === "l") { n++; } }
console.log(n);
