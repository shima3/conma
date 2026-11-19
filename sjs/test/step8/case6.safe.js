// 配列の分割代入：配列は添字で、その時の配列から読む（JavaScript の配列の繰り返しと同じ）。ほかは繰り返しの手順
const pairs = [[1, 2], [3, 4]];
for (const [a, b] of pairs) console.log(JSON.stringify([a, b]));
const [x, , z = 9, ...r] = [1, 2, undefined, 4, 5];
console.log(JSON.stringify([x, z, r]));
const [p, q] = [7];
console.log(JSON.stringify([p, q]));
const arr = [1, 2, 3];
const [u = arr.push(10), ...rest] = [undefined].concat(arr);
console.log(JSON.stringify([u, rest, arr.length]));
const live = [1, 2];
const [m = (live.push(3), 0), , n] = live.length ? [undefined, 5] : [];
console.log(JSON.stringify([m, n]));
const [c1, c2] = "hé!";
console.log(JSON.stringify([c1, c2]));
function* g() { try { yield 1; yield 2; yield 3; } finally { console.log("closed"); } }
const [g1, g2] = g();
console.log(JSON.stringify([g1, g2]));
const [h1, ...hs] = g();
console.log(JSON.stringify([h1, hs]));
let s1, s2;
[s1, s2] = [s2, s1] = [10, 20];
console.log(JSON.stringify([s1, s2]));
const holes = [1, , 3];
const [k0, k1, k2, k3] = holes;
console.log(JSON.stringify([k0, k1, k2, k3]));
const [[d1, d2], [d3]] = [[1, 2], [3]];
console.log(JSON.stringify([d1, d2, d3]));
try { const [e1] = 5; } catch (e) { console.log("TypeError"); }
try { const [e1] = null; } catch (e) { console.log("TypeError"); }
const src = [1, 2, 3];
const [sa, ...sr] = src;
sr.push(4); console.log(JSON.stringify([sa, sr, src]));
const big = []; big[5] = "x";
const [, , ...br] = big; console.log(JSON.stringify([br.length, br[3]]));
const la = [undefined];
const [l1 = la.push(8), l2] = la;
console.log(JSON.stringify([l1, l2]));
