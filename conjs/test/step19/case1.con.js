// 名前付きの import と export、as による名前の付け替え、export { }、関数とクラス
import { PI, area, Box, reveal, count as n } from "./mods/shapes.con.js"
import { area as a2 } from "./mods/shapes.con.js"
console.log(PI, area(2), a2(1), new Box(3).size(), reveal())
console.log(n)
