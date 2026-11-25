"use strict";
// ?hostreport in the URL of a page of ConJS (docs/dev/CONJS-HOST-PLAN.md, section 32; stage H7): the VM gets
// CONJS_HOST_REPORT and CONJS_HOST_STRICT, and the lines they write to the standard error (conjs: host: NAME,
// conjs: off: NAME, conjs: escape: NAME, and the errors of the JavaScript of the host functions) go to the console of
// the browser, not into the page. Other output goes where the page writes it.
//   const report = ConjsHostReport();
//   const vm = ConMaVM.create(module, report.wrap(write), hooks);
//   report.setup(vm);   // before vm.start
const ConjsHostReport = () => {
  const on = new URLSearchParams(location.search).has("hostreport");
  const wrap = (write) => {
    if (!on) return write;
    const decoder = new TextDecoder();
    const encoder = new TextEncoder();
    let rest = "";
    return (fd, bytes) => {
      if (fd !== 2) return write(fd, bytes);
      rest += decoder.decode(bytes, { stream: true });
      let other = "";
      for (let i = rest.indexOf("\n"); i >= 0; i = rest.indexOf("\n")) {
        const line = rest.slice(0, i);
        rest = rest.slice(i + 1);
        if (/^conjs: (host|off|escape): /.test(line)) console.info(line);
        else other += line + "\n";
      }
      if (other !== "") write(fd, encoder.encode(other));
    };
  };
  const setup = (vm) => {
    if (!on) return;
    vm.setenv("CONJS_HOST_REPORT", "1");
    vm.setenv("CONJS_HOST_STRICT", "1");
  };
  return { on, wrap, setup };
};
