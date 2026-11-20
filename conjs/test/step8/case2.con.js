// 読めないファイルと、引数のエラー
try { fs.readFile("no-such-file.txt"); } catch (e) { console.log(e.name, e.message.startsWith("fs.readFile: ")); }
try { fs.writeFile("no-such-dir/x.txt", "x"); } catch (e) { console.log(e.name, e.message.startsWith("fs.writeFile: ")); }
try { fs.readFile(5); } catch (e) { console.log(e.name, e.message); }
try { fs.writeFile("out.txt", 5); } catch (e) { console.log(e.name, e.message); }
