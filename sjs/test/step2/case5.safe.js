// 組み込み：Math、Number、String、parseInt、isNaN、Array.isArray、typeof、console.log の形
console.log(Math.max(1, 5, 3), Math.min(), Math.floor(2.7), Math.ceil(2.1), Math.round(2.5), Math.abs(-3), Math.sqrt(16), Math.trunc(-2.7), Math.sign(-3), Math.PI)
console.log(Number("42") + 1, Number(""), Number("x"), String(5), String([1, 2]), parseInt("12px"), parseInt("x"), isNaN("x"), Number.isInteger(3))
console.log(Array.isArray([]), Array.isArray({}), typeof [], typeof {}, typeof null, Boolean(""), Boolean("a"))
console.log([1, [2, [3, [4]]]], { a: { b: { c: { d: 1 } } } }, [], {}, ["it's"], [`a"b`])
console.log([undefined, null, true, 1.5], { "key with space": 1, valid_key: 2 })
