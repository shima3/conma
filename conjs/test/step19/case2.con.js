// 同じファイル（log.con.js）を 3 つのファイルが、違う書き方の指定で読み込んでも、1 度だけ、深さ優先の帰りがけの順で動く
import { fromB } from "./mods/deep/b.con.js"
import { lines } from "./mods/./deep/../log.con.js"
import { fromA } from "./mods/a.con.js"
console.log("case2 runs", fromA, fromB, lines.join(","))
