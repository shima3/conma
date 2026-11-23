export function fail(x) {
  if (x > 1) {
    throw new Error(`too big: ${x}`)
  }
  return x
}
