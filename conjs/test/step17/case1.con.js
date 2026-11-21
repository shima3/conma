// 決まらない Promise（段階 P7。docs/dev/PLATFORM-PLAN-ja.md の 4.9 節）：wasm 版の VM で、ホストの関数（case1.host.js）が返した
// Promise がいつまでも決まらず、ほかにすることもなければ、vm-node.js が行き詰まりを知らせて、終了ステータス 2 で終える
// （前は、何も出さずに 0 で終わった）。ほかの VM には、その関数がない
console.log("start");
setTimeout(() => console.log("a task that ends"), 10);
try { host.wait("demo.never"); } catch (e) { console.log(String(e)); }
console.log("end");
