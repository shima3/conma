// join されないまま例外で終わった処理は、すべての処理が終わってから、終わった順に報告する（終了ステータス 2）
spawn(() => { host.wait("test.later", 2); throw "second"; });
spawn(() => { host.wait("test.later", 1); undefined.y; });
const ok = spawn(() => 42);
const late = spawn(() => { throw "joined later"; });
host.wait("test.later", 5);
try { join(late); } catch (e) { console.log("late:", e); }
console.log("main end", join(ok));
