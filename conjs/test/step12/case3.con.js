// ふつうの function の this は、変換するときのエラー
class A {
  m() {
    function inner() { return this; }
    return inner();
  }
}
