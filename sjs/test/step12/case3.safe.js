// JSON.parse は、JSON の数の形でないものと、文字列の中の制御文字を、SyntaxError にする（M2 のレビュー）
for (const s of ["+1", "01", "1.", ".5", "\"a\tb\"", "\"a\nb\"", "-", "1e", "1e+", "-01", "0x1", "-0", "0.5e-3", "12", "[1,-2.5E2,0]", "\"\\u0041\\n\\t\"", "1E+2"]) {
  try { console.log(JSON.stringify(JSON.parse(s))); } catch (e) { console.log(e.name); }
}
