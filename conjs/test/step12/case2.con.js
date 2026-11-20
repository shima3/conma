// 受け手のないエラー（Node.js と比べない。標準エラーとステータスが違う）
class AppError extends Error {}
console.log("before");
throw new AppError("unhandled");
