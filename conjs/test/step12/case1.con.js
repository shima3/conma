// エラーのコンストラクタと instanceof（4.5 節）
const errors = [new Error("e"), new TypeError("t"), new RangeError("r"), new SyntaxError("s"), new ReferenceError("f"), Error("no new")];
for (const e of errors) {
  console.log(e.name, e.message, `${e}`, e instanceof Error, e instanceof TypeError, e instanceof RangeError);
}
console.log(`${new Error()}`, new Error().message === "", Object.keys(new Error("x")), JSON.stringify(new Error("x")));

class AppError extends Error {
  constructor(message, code) {
    super(message);
    this.code = code;
  }
  describe() { return `${this.message} (${this.code})`; }
}
class NotFound extends AppError {
  constructor(what) { super(`${what} not found`, 404); }
}
try {
  throw new NotFound("page");
} catch (e) {
  console.log(e.message, e.code, e.describe(), `${e}`);
  console.log(e instanceof NotFound, e instanceof AppError, e instanceof Error, e instanceof TypeError);
}

// ランタイムが投げるエラーも instanceof で分けられる
for (const f of [() => null.x, () => undefined(), () => { throw new RangeError("out"); }]) {
  try {
    f();
  } catch (e) {
    if (e instanceof TypeError) console.log("TypeError:", e.message);
    else if (e instanceof RangeError) console.log("RangeError:", e.message);
  }
}
