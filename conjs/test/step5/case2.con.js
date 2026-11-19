// 使い方のエラー。最初の行は、me の中の join(me) が投げた TypeError を、main の join(me) が投げ直したもの
let me;
me = spawn(() => join(me));
try { join(me); } catch (e) { console.log("main:", String(e)); }
try { spawn(42); } catch (e) { console.log(String(e)); }
try { join({}); } catch (e) { console.log(String(e)); }
try { host.call(1); } catch (e) { console.log(String(e)); }
try { host.wait("test.later", true); } catch (e) { console.log(String(e)); }
try { host.call("test.echo", { a: 1 }); } catch (e) { console.log(String(e)); }
console.log("typeof:", typeof me, JSON.stringify(me), String(me), "x" in { x: me });
