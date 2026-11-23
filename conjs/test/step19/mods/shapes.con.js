// import の見本（conjs/test/step19）：いろいろな export の形
export const PI = 3
export let count = 0
export function area(r) { count = count + 1; return PI * r * r }
export class Box {
  constructor(w) { this.w = w }
  size() { return this.w * this.w }
}
const secret = "s"
function hidden() { return secret }
export { hidden as reveal }
