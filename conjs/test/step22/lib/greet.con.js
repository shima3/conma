// case1.con.js が import する。async 関数は、この場所から require し、名前は case1.con.js の同じ名前の関数と混ざらない
export async function greet(name) {
  const { twice } = require("./helper.js");
  return `hello, ${name} (${twice(21)})`;
}

export async function which() {
  return "greet.con.js";
}
