// main の受け手のない例外は、ほかの処理を待たずに、すぐに終える
spawn(() => { host.wait("test.later", 1); console.log("never"); });
spawn(() => console.log("runs first"));
host.wait("test.later", 0);
console.log("main throws");
null.z;
