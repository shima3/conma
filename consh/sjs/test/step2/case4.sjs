// テンプレート文字列と、文字列のメソッド
const name = "world"
console.log(`hello ${name}!`, `${1 + 2} and ${[1, 2]} and ${null} and ${{}}`, `nested ${`in ${name}`}`)
console.log(`multi
line`, `esc \` \${x} \t.`)
console.log("a,b,c".split(","), "abc".split(""), "Hello".toUpperCase(), "Hello".toLowerCase(), "  t  ".trim())
console.log("abcdef".slice(1, 3), "abcdef".slice(-2), "abc".indexOf("c"), "abc".includes("b"), "abc".startsWith("ab"), "abc".endsWith("bc"))
console.log("ab".repeat(3), "abc".charAt(1), "abc"[2], "abc".length)
