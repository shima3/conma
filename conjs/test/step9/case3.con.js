// つながらない所は TypeError（JavaScript と同じ）。引数のエラーも TypeError
try { fetch("http://127.0.0.1:1/"); } catch (e) { console.log(e.name, e.message.startsWith("fetch failed")); }
try { fetch(5); } catch (e) { console.log(e.name, e.message); }
try { fetch("http://127.0.0.1:1/", { headers: 5 }); } catch (e) { console.log(e.name, e.message); }
try { fetch("http://127.0.0.1:1/", { method: 1 }); } catch (e) { console.log(e.name, e.message); }
