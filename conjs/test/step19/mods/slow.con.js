// 読み込まれるファイルの最上位で待つ（ConJS だけ。JavaScript の最上位の await と同じ振る舞い）
console.log("slow starts")
sleep(10)
const t = spawn(() => { sleep(5); return "task done" })
export const ready = join(t)
console.log("slow ends")
