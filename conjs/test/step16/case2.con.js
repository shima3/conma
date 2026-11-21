// 書く（4.8 節、段階 P8）："w" は作るか空にし、"a" は後ろに足す。閉じた後と、開けないファイル、読むだけのファイルへの
// write、知らない mode のエラー
const w = fs.open("out.txt", "w");
w.write("first\n");
w.write("second\n");
w.close();
w.close();                                   // 何度閉じてもよい
const a = fs.open("out.txt", "a");
a.write("third");
a.close();
console.log(JSON.stringify(fs.readFile("out.txt")));
console.log(JSON.stringify([...fs.readLines("out.txt")]));
try { w.write("more"); } catch (e) { console.log(`${e}`); }
try { fs.open("no-such-dir/x.txt", "w"); } catch (e) { console.log(e.name); }
try { fs.open("no-such-file.txt"); } catch (e) { console.log(e.name); }
try { fs.open("out.txt").write("x"); } catch (e) { console.log(`${e}`); }
try { fs.open("out.txt", "w").readLine(); } catch (e) { console.log(`${e}`); }
try { fs.open("out.txt", "rw"); } catch (e) { console.log(`${e}`); }
try { fs.open(1); } catch (e) { console.log(`${e}`); }
