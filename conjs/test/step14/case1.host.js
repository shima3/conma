// The input of case1.con.js (input_mock.js): what happens after each call of input.next
const InputMock = require("./input_mock.js");
module.exports = InputMock([
  [["key", "ArrowUp"], ["click", 10, 20]],   // 1: two events; the second stays in the queue
  [["advance", 50]],                         // 2: nextEvent(30) times out
  [["advance", 0]],                          // 3: nextEvent(-5) is nextEvent(0)
  [["advance", 0]],                          // 4: nextEvent(NaN) too
  [["advance", 10], ["key", "x"]],           // 5: nextEvent(10) times out, then x comes and stays
  [],                                        // 6: the main task waits
  [["key", "a"], ["key", "b"]],              // 7: the spawned task waits too; the first wait takes a
  [["key", "Escape"]],                       // 8: nextEvent(Infinity) does not time out
]);
