// 引数のエラーと、機能の有無
try { sleep("x"); } catch (e) { console.log(e.name, e.message); }
try { setTimeout(1, 2); } catch (e) { console.log(e.name, e.message); }
try { setTimeout(() => 1, "soon"); } catch (e) { console.log(e.name, e.message); }
console.log(typeof sleep, typeof setTimeout, typeof Date.now, typeof fs.readFile, typeof fetch);
console.log("time:", host.available("time"));
