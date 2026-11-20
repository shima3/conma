// キーが constructor や __proto__ のオブジェクト：JSON.parse は、そのままのプロパティにする（プロトタイプはない）。
// Object.keys には出るが、読むと TypeError（Node.js と比べない。JavaScript では読める）
const o = JSON.parse('{"constructor": 1, "__proto__": 2, "x": 3}');
console.log(Object.keys(o), JSON.stringify(o));
for (const k in o) {
  try {
    console.log(k, o[k]);
  } catch (e) {
    console.log(k, e.name, e.message);
  }
}
