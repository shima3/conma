function* g() { yield 1; }
for await (const v of g()) console.log(v);
