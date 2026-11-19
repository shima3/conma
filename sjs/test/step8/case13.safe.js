// Number() of strings and trim() (sjs_rt.se の Sjs_decimal_text と Sjs_trim。2026-11-14 から、文字の LList を作らずに見る)。
// 出力は Node.js と同じ。0x10、+Infinity、1e400 は SJS では NaN になり、Node.js と違うので比べない
const xs = ["", " ", "0", "-0", "+0", "12", "-12", "+12", "1.", ".5", "-.5", "+.5e-3", "1.5e10", "1.5E+10", "2e-3",
  "4.84143144246472090e+00", ".", "-", "+", "e5", "1e", "1e+", "1e-", "1.2.3", "1e5.5", "1ee5", "--1", "+-1", "1-",
  "12a", "a12", "1_000", "Infinity", "-Infinity", " 12 ", "\t12\n", "\r-3.5 ", " 1 2 ", "  ", "\n", "NaN", "inf",
  "١٢", "1 ", " 1", "00012", "1e0010", "9007199254740993", "-1e-400", "1.e5", ".e5", "5.e"]
for (const x of xs) console.log(JSON.stringify(x), Number(x))
console.log(`${" a b ".trim()}|${"ab".trim()}|${"".trim()}|${" ".trim()}|${"a ".trim()}|${"\ta".trim()}|${" x\t\n".trim()}|${"a".trim()}|`)
