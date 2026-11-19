// 関数：アロー関数、function 式、足りない引数、値のない return、入れ子
const add = function (p, q) { return p + q }
console.log(add(1, 2))
const second = (p, q) => q
console.log(second(1) === undefined, second(1, 2, 3))
const nothing = () => { return }
console.log(nothing(), (() => {})())
const compose = (f, h) => (v) => f(h(v))
const inc = (v) => v + 1
const dbl = (v) => v * 2
console.log(compose(inc, dbl)(5), compose(dbl, inc)(5))
const early = (v) => { if (v > 0) { return "positive" } else if (v < 0) { return "negative" } return "zero" }
console.log(early(3), early(-3), early(0))
const counter = () => { let k = 0; while (true) { k++; if (k >= 5) { return k } } }
console.log(counter())
