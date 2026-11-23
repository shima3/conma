// ライブバインディング：出した側が変えた値が、読み込んだ側にも見える
import { value, bump, reset } from "./mods/counter.con.js"
console.log(value)
bump()
bump()
console.log(value)
reset()
console.log(value)
