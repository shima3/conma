// wasm 版の VM：CONMA_HOST の関数が Promise を返す（case4.host.js）。ほかの VM には、その関数がない
if (host.available("demo")) {
  const t = spawn(() => host.wait("demo.slow_sum", 2, 3));
  console.log("sum:", host.call("demo.sum", 1, 2));
  console.log("slow sum:", join(t));
  try { host.wait("demo.reject"); } catch (e) { console.log(String(e)); }
  try { host.wait("demo.slow_bool"); } catch (e) { console.log(String(e)); }
} else {
  console.log("no demo host");
}
