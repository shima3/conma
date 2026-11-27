// Queue を作るだけでも、ホストの JavaScript がなければ始めで止まる（D15。run_all.sh が new Function を止めて流す）
console.log("printed with the host");
const q = new Queue();
console.log(q.closed);
