// dict: an object as a dictionary, counting N_ARG words of three syllables (512 kinds), then the 5 most frequent
const n = N_ARG
const syll = ["ka", "ki", "ku", "ke", "ko", "sa", "shi", "su"]
const counts = {}
let seed = 1
for (let i = 0; i < n; i++) {
  seed = (seed * 75 + 74) % 65537
  const w = `${syll[seed % 8]}${syll[Math.floor(seed / 8) % 8]}${syll[Math.floor(seed / 64) % 8]}`
  counts[w] = counts[w] === undefined ? 1 : counts[w] + 1
}
const words = Object.keys(counts)
words.sort((a, b) => counts[b] - counts[a] || (a < b ? -1 : 1))
console.log(words.length)
for (const w of words.slice(0, 5)) console.log(`${w} ${counts[w]}`)
