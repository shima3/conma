// Run the ConMa VM for wasm32 in the JavaScriptCore shell, as conma (vm/src/main.rs) runs:
//   jsc vm-jsc.js -- <dir of consh/js> <current directory> <file of `env -0`> <file of dirs> <file of files>
//       [-t] [-m] <module_file> <func_name | arg ...>
// The VM starts in the current directory, with the environment variables of the file. The virtual file
// system has the directories and the files of the two files (paths separated by NUL, from `find -print0`
// in the current directory); a file is read from the real file system when the VM opens it, also one
// that is not listed (the shell cannot list directories).
// The shell has no way to read or write bytes or to set the exit code, so vm-wasm.sh gives standard
// input as lines "d <hex bytes>" and "e" at its end, read only when every VProc waits, and each
// write is printed as a line "<fd> <hex bytes>", and the end as "x <exit code>", which vm-wasm.sh
// turns back into bytes.
(function () {
  load(arguments[0] + "/conma.js");
  // The shell gives each byte of an argument as a character.
  const [jsdir, cwd, envFile, dirsFile, filesFile, ...argv] = Array.from(arguments, (a) =>
    ConMaVM.text(Uint8Array.from(a, (c) => c.charCodeAt(0)))
  );
  const hex = (b) => Array.from(b, (c) => (c < 16 ? "0" : "") + c.toString(16)).join("");
  const wasm = new WebAssembly.Module(read(jsdir + "/wasm/target/wasm32-unknown-unknown/release/conma_wasm.wasm", "binary"));
  const unhex = (h) => Uint8Array.from(h.match(/../g) || [], (b) => parseInt(b, 16));
  let inputEnded = false;
  // Give the next piece of standard input (waiting for it), or its end.
  function readInput(vm) {
    if (inputEnded) return;
    const line = readline();
    if (line.startsWith("d ")) {
      vm.input(unhex(line.slice(2)));
    } else {
      vm.inputEnd();
      inputEnded = true;
    }
  }
  const fetchFile = (vm, path) => {
    try {
      vm.putFile(path, read(path, "binary"));
    } catch (e) {
      // not in the real file system either, or a directory
    }
  };
  const vm = ConMaVM.create(wasm, (fd, bytes) => print(fd + " " + hex(bytes)), readInput, fetchFile);
  // The shell cannot tell directories: a directory on the way of a path is taken to exist.
  vm.assumeDirs();
  vm.makeDirs(cwd);
  const paths = (file) => (file === "" ? [] : ConMaVM.text(read(file, "binary")).split("\0").filter((p) => p !== ""));
  const abs = (p) => (p.startsWith("/") ? p : cwd + "/" + p);
  for (const p of paths(dirsFile)) vm.makeDirs(abs(p));
  for (const p of paths(filesFile)) vm.putUnread(abs(p));
  vm.setCwd(cwd);
  if (envFile !== "") {
    for (const entry of ConMaVM.text(read(envFile, "binary")).split("\0")) {
      const i = entry.indexOf("=");
      if (i > 0) vm.setenv(entry.slice(0, i), entry.slice(i + 1));
    }
  }
  // The module file named in argv, read here as the native VM reads it.
  let i = 0;
  // As conma does: options before the module file, up to "--"
  while (i < argv.length && ["-t", "--trace", "-m", "--main"].includes(argv[i])) i++;
  if (argv[i] === "--") i++;
  let module = new Uint8Array(0);
  try {
    if (i < argv.length) module = read(argv[i], "binary");
  } catch (e) {
    print("2 " + hex(ConMaVM.utf8("Error: [Errno 2] No such file or directory: '" + argv[i] + "'\n")));
    print("x 1");
    return;
  }
  let code = vm.start(module, argv);
  if (code === 0) {
    for (;;) {
      code = vm.run(100000);
      // A program may poll standard input instead of waiting for it: give it input when it found none.
      if (code === ConMaVM.RUNNING && !(vm.wantsInput() && !inputEnded)) continue;
      if (code !== ConMaVM.RUNNING && code !== ConMaVM.IDLE) break;
      if (!inputEnded) {
        readInput(vm);
        continue;
      }
      if (code === ConMaVM.IDLE) {
        // Input has ended and there are no processes: nothing can wake the VM
        // (the native VM would wait for ever).
        print("2 " + hex(ConMaVM.utf8("vm-jsc: every VProc waits, and nothing can wake them\n")));
        code = 2;
      }
      break;
    }
  }
  print("x " + code);
})(...arguments);
