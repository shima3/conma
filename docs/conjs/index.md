---
title: ConJS
---

# ConJS

ConJS is a subset of JavaScript with lightweight threads that switch cooperatively and with blocking calls (conjs/ of the repository): a call that waits, such as a function of the host or a frame of an animation, is written as an ordinary call, with no `await`. It is made with the translator and the runtime of SJS, a safe subset of JavaScript (sjs/ of the repository), which ConSh uses too. The documents of ConMa are in [the index of the documents](../).

The documents of ConJS for its users are listed only here; a new one is added here. They are copied from the repository when the site is built: conjs/README.md, conjs/README-ja.md, conjs/QUICKSTART.md, conjs/QUICKSTART-ja.md, conjs/TUTORIAL.md, conjs/TUTORIAL-ja.md and sjs/README-ja.md.

## Docs
- Quick start ([English](QUICKSTART.html), [日本語](QUICKSTART-ja.html)): install the tools, clone, build, and run a web server written in ConJS on Node.js
- Tutorial ([English](TUTORIAL.html), [日本語](TUTORIAL-ja.html)): the JavaScript that ConJS takes, and where it differs from JavaScript, by examples
- ConJS ([English](README.html), [日本語](README-ja.html)): how to run it, the language and how it differs from JavaScript, the examples
- [SJS](../sjs/README-ja.html) (in Japanese): the safe subset of JavaScript that ConJS and ConSh are built on
- [Functions of the host](../HOST-ja.html) (in Japanese): how to add functions of the host, which a program calls through `host`

## In your browser
These run ConJS programs on the wasm VM (conjs/web of the repository).
- [Stars](web/stars.html) (星ひろい): a game that waits for input (conjs/web/stars.con.js). The arrow keys or a click move you; pick up every star and keep away from the ghosts; Escape ends it. The same game in plain JavaScript: [event-driven](web/stars-events.html), [with async/await](web/stars-async.html); [compare the code](web/compare.html) of the three
- [Canvas](web/canvas.html): a bouncing ball (conjs/web/canvas_demo.con.js)

## For developers of ConJS

The documents for those who change ConJS are not published here: they are read on GitHub, in the branch `alpha`, where they are newer ([docs/dev](https://github.com/shima3/conma/tree/alpha/docs/dev), beginning with [How ConJS is made, and its tests](https://github.com/shima3/conma/blob/alpha/docs/dev/CONJS.md) and [the plan of the platform](https://github.com/shima3/conma/blob/alpha/docs/dev/PLATFORM-PLAN-ja.md), sections 4 and 8).
