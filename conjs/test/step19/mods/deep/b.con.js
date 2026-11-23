import { note } from "../log.con.js"
import { fromA } from "../a.con.js"
console.log("b.con.js runs")
note("b")
export const fromB = `B after ${fromA}`
