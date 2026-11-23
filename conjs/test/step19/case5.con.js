// 読み込まれるファイルの関数で投げた例外の位置は、そのファイルの名前と行と桁
import { fail } from "./mods/thrower.con.js"
console.log(fail(1))
fail(2)
