// ?. は鎖全体を undefined にする
const a = null, o = { b: { c: 5, f: () => 7 }, n: null, arr: [1, 2] }
console.log(a?.b.c, a?.b.c.d.e, o?.b.c, o.n?.x.y, o.b?.f(), o.b.g?.(), o.n?.f(), a?.[1].x, o.arr?.[1])
console.log(o.zz?.q.r ?? "default", (o.n?.x)?.y)
try { (o.n?.x).y } catch (e) { console.log(e.message) }
const deep = { x: { y: null } }
console.log(deep.x.y?.z.w, deep?.x?.y?.z, [a?.b, o?.b?.c].join(","))
