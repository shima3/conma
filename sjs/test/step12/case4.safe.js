// 配列の at と、文字列の replace、replaceAll（2026-11-24。ConJS と共有する）。ログは Node.js の結果と比べた
const a = [10, 20, 30];
console.log(a.at(0), a.at(2), a.at(-1), a.at(-3), a.at(3), a.at(-4), a.at(), a.at(1.7), a.at(-1.2), a.at("1"), a.at(NaN), a.at(Infinity), a.at(-Infinity));
console.log([].at(0), [1, , 3].at(1), ["x"].at(-0));
const s = "a-b-c";
console.log(s.replace("-", "+"), s.replaceAll("-", "+"), s.replace("x", "y"), s.replaceAll("x", "y"));
console.log("abc".replace("", "-"), "abc".replaceAll("", "-"), "".replaceAll("", "-"), "".replace("", "-"));
console.log("aaa".replaceAll("aa", "b"), "aaaa".replaceAll("aa", "b"));
console.log("x=1".replace("=", "[$&]"), "abc".replace("b", "$`|$'"), "a$b".replace("$", "$$"), "ab".replace("a", "$1$<n>$"), "ab".replace("a", "$"));
console.log("a-b-c".replaceAll("-", (m, i, str) => `(${m}${i}${str.length})`));
console.log("a-b".replace("-", () => 42), "a-b".replace("-", 7), "a1b1".replaceAll(1, 2), "nullx".replace(null, "N"));
console.log("héllo wörld".replaceAll("ö", "o"), "日本語の本".replaceAll("本", "ホン"));
console.log([1, 2, 3].at(-1) + "abc".replaceAll("b", "B").length);
