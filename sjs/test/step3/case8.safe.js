// 段階 B9：catch して投げ直した例外と、finally を通った例外は、最初に投げた所の位置
function risky() {
  try {
    throw { message: "first" }
  } finally {
    console.log("cleanup")
  }
}
try {
  risky()
} catch (e) {
  console.log(`caught ${e.message}`)
}
const later = () => { risky() }
later()
