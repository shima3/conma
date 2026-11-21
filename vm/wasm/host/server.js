// The functions of the host for a network server in Node.js (feature "server"; docs/dev/PLATFORM-PLAN-ja.md 4.10,
// stage P6b). vm-node.js gives them to every program, with those of CONMA_HOST; the browser has none.
//   server.listen port host      (waits) starts an HTTP server on host ("127.0.0.1" if empty) and port (0: the OS
//                                chooses), and gives {id, port}; a port that cannot be used is an error
//                                ("server.listen: EADDRINUSE: address already in use")
//   server.accept id             (waits) the next request, {id, method, url, headers, body}, or null once the server
//                                is closed. Requests that come before anyone accepts them wait in a queue. headers are
//                                those of Node.js (req.headers); body is the whole body as a string (UTF-8). A body over
//                                1 MB is answered with 413 and does not go to the queue (over 64 MB, the connection is
//                                closed)
//   server.respond requestId status headers body
//                                answers the request (writeHead and end); a second answer is an error
//   server.close id              stops taking requests, as server.close() of Node.js: requests already accepted can
//                                still be answered, those in the queue are answered with 503, and a waiting
//                                server.accept gets null. Idle keep-alive connections are closed
// The server keeps the event loop of Node.js alive, so a program that waits in server.accept waits until the server
// is closed or the process gets SIGINT; it is not a deadlock.
"use strict";
const http = require("http");

const BODY_LIMIT = 1 << 20;

function serverHost() {
  const servers = new Map(); // id -> {srv, queue, waiters, closed}
  const requests = new Map(); // id -> {res, server}
  let nextServer = 0;
  let nextRequest = 0;

  const answer = (res, status, text) => {
    try {
      res.writeHead(status, { "content-type": "text/plain; charset=utf-8", connection: "close" });
      res.end(text);
    } catch (e) {
      // the client has gone
    }
  };

  function onRequest(s, req, res) {
    if (s.closed) {
      answer(res, 503, "Service Unavailable\n");
      return;
    }
    const chunks = [];
    let size = 0;
    let tooLarge = false;
    // A body over the limit is read to its end without being kept, and answered then (answering while the client
    // still sends would reset the connection before the client reads the answer); one far over it closes the connection
    req.on("data", (d) => {
      size += d.length;
      if (size > BODY_LIMIT) {
        tooLarge = true;
        chunks.length = 0;
        if (size > 64 * BODY_LIMIT) req.destroy();
        return;
      }
      chunks.push(d);
    });
    req.on("end", () => {
      if (tooLarge) {
        answer(res, 413, "Payload Too Large\n");
        return;
      }
      if (s.closed) {
        answer(res, 503, "Service Unavailable\n");
        return;
      }
      const id = ++nextRequest;
      requests.set(id, { res, server: s });
      const r = { id, method: req.method, url: req.url, headers: Object.assign({}, req.headers), body: Buffer.concat(chunks).toString("utf8") };
      if (s.waiters.length > 0) s.waiters.shift()(r);
      else s.queue.push(r);
    });
    req.on("error", () => {});
  }

  const getServer = (what, id) => {
    const s = servers.get(id);
    if (!s) throw new Error(`${what}: no server ${id}`);
    return s;
  };

  const hostFunctions = {
    "server.listen": (port, host) => {
      if (typeof port !== "number" || !Number.isInteger(port) || port < 0 || port > 65535)
        throw new Error("server.listen: the port must be an integer from 0 to 65535");
      if (host !== undefined && typeof host !== "string") throw new Error("server.listen: the host must be a string");
      return new Promise((resolve, reject) => {
        const s = { srv: null, queue: [], waiters: [], closed: false };
        const srv = http.createServer((req, res) => onRequest(s, req, res));
        s.srv = srv;
        srv.once("error", (e) => {
          const m = String(e.message).replace(/^listen /, "").replace(/ \S+:\d+$/, "");
          reject(new Error(`server.listen: ${m}`));
        });
        srv.listen(port, host || "127.0.0.1", () => {
          const id = ++nextServer;
          servers.set(id, s);
          resolve({ id, port: srv.address().port });
        });
      });
    },
    "server.accept": (id) => {
      const s = getServer("server.accept", id);
      if (s.queue.length > 0) return s.queue.shift();
      if (s.closed) return null;
      return new Promise((resolve) => s.waiters.push(resolve));
    },
    "server.respond": (id, status, headers, body) => {
      const r = requests.get(id);
      if (!r) throw new Error("server.respond: the request has been answered");
      if (typeof status !== "number" || !Number.isInteger(status) || status < 100 || status > 999)
        throw new Error("server.respond: the status must be an integer from 100 to 999");
      if (typeof body !== "string") throw new Error("server.respond: the body must be a string");
      requests.delete(id);
      const h = headers && typeof headers === "object" && !Array.isArray(headers) ? headers : {};
      try {
        if (r.server.closed) r.res.setHeader("connection", "close");
        r.res.writeHead(status, h);
        r.res.end(body);
      } catch (e) {
        throw new Error(`server.respond: ${e.message}`);
      }
      return null;
    },
    "server.close": (id) => {
      const s = getServer("server.close", id);
      if (s.closed) return null;
      s.closed = true;
      s.srv.close();
      for (const r of s.queue.splice(0)) {
        const rec = requests.get(r.id);
        requests.delete(r.id);
        if (rec) answer(rec.res, 503, "Service Unavailable\n");
      }
      for (const w of s.waiters.splice(0)) w(null);
      s.srv.closeIdleConnections();
      return null;
    },
  };
  return { hostFunctions, hostFeatures: ["server"] };
}

module.exports = { serverHost };
