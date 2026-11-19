// 文字列、比べること、真偽
console.log("a" < "b", "abc" < "abd", "b" > "abc", "a" <= "a", 2 < 10, 10 >= 10)
console.log(3 === 3, 3 === "3", "x" !== "y", null === null, undefined === null, true === true)
console.log(typeof 1, typeof "s", typeof true, typeof undefined, typeof null, typeof (() => 1))
console.log(!0, !1, !"", !"a", !null, !undefined)
console.log(0 || "zero", 1 && "one", null ?? "nullish", 0 ?? "kept", "" || null)
console.log(1 > 2 ? "yes" : "no", true ? false ? 1 : 2 : 3)
console.log("tab\tand\\backslash", 'single "quotes"', "Aあ")
