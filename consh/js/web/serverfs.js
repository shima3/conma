// The file system of ConSh in a browser (hostFs of conma.js): the files of the server, beside the
// page, are under home (/home/user, or / and others, see consh.js), read from the server each time
// they are opened or listed, and the files written are kept in memory over them (they go when the
// page is closed). The directories on the way to home, /dev and /dev/null are in memory, and come
// before the files of the server of the same names (when home is /). The browser
// keeps the files of the server in its cache as it does other files (serve.py asks it to check them
// with the server each time, Cache-Control: no-cache, so that a file changed on the disk is seen).
//
// The server is asked with synchronous XMLHttpRequest, since the VM opens and lists files
// synchronously. Whether a name is a directory: a directory without / at the end is redirected to the
// name with / (as python3 -m http.server and most servers do). The entries of a directory: serve.py
// gives them for ?consh-ls (as JSON, the names of directories ending with /); for another server, the
// links of its listing of the directory (python3 -m http.server gives one, unless the directory has an
// index.html) are taken, and else the directory seems empty.
//
// All functions take absolute paths and return minus an errno on an error.
const ServerFS = (function () {
  "use strict";
  const ENOENT = -2, EIO = -5, EBADF = -9, ENOTDIR = -20, EISDIR = -21, EINVAL = -22;

  // A path without . and .., and without / at the end (null if it is not absolute).
  function normalize(path) {
    if (!path.startsWith("/")) return null;
    const parts = [];
    for (const c of path.split("/")) {
      if (c === "" || c === ".") continue;
      if (c === "..") parts.pop();
      else parts.push(c);
    }
    return "/" + parts.join("/");
  }
  const parentOf = (p) => p.slice(0, p.lastIndexOf("/")) || "/";

  function create(home, base) {
    base = new URL(base || ".", location.href).href; // the directory of the page
    if (!base.endsWith("/")) base += "/";
    // path -> {dir: true} | {data: Uint8Array}; /dev/null is {null: true}
    const mem = new Map([["/", { dir: true }], ["/dev", { dir: true }], ["/dev/null", { null: true }]]);
    for (let p = home; p !== "/"; p = parentOf(p)) mem.set(p, { dir: true });

    const homePrefix = home === "/" ? "/" : home + "/";
    const under = (p) => p.startsWith(homePrefix);
    // The URL of a path under home (null for another path).
    function urlOf(p, dir) {
      if (p === home) return base;
      if (!under(p)) return null;
      return base + p.slice(homePrefix.length).split("/").map(encodeURIComponent).join("/") + (dir ? "/" : "");
    }
    function request(method, url) {
      const x = new XMLHttpRequest();
      x.open(method, url, false);
      // The bytes as they are (a synchronous request cannot have responseType arraybuffer)
      x.overrideMimeType("text/plain; charset=x-user-defined");
      try {
        x.send();
      } catch (e) {
        return null;
      }
      return x;
    }
    const bytesOf = (s) => {
      const b = new Uint8Array(s.length);
      for (let i = 0; i < s.length; i++) b[i] = s.charCodeAt(i) & 0xff;
      return b;
    };
    // 1 for a directory, 2 for a file of the server, or minus an errno; with the bytes of a file
    // when get is true ({kind, data}).
    function server(p, get) {
      const url = urlOf(p, false);
      if (url === null) return { kind: ENOENT };
      const x = request(get ? "GET" : "HEAD", url);
      if (x === null) return { kind: EIO };
      if (x.status === 404 || x.status === 403) return { kind: ENOENT };
      if (x.status < 200 || x.status >= 300) return { kind: EIO };
      if (url.endsWith("/") || x.responseURL.endsWith("/")) return { kind: 1 };
      return { kind: 2, data: get ? bytesOf(x.responseText) : null };
    }

    // 1 for a directory, 2 for another file, or minus an errno (ENOTDIR for a file on the way).
    function kind(p) {
      const m = mem.get(p);
      if (m) return m.dir ? 1 : 2;
      if (p !== "/") {
        const k = kind(parentOf(p));
        if (k < 0) return k;
        if (k !== 1) return ENOTDIR;
      }
      return server(p, false).kind;
    }

    // The names in the directory p (an array, or minus an errno).
    function list(p) {
      const k = kind(p);
      if (k < 0) return k;
      if (k !== 1) return ENOTDIR;
      const names = new Set();
      const prefix = p === "/" ? "/" : p + "/";
      for (const q of mem.keys()) if (q.startsWith(prefix) && q.length > prefix.length && !q.slice(prefix.length).includes("/")) names.add(q.slice(prefix.length));
      const url = urlOf(p, true);
      if (url !== null) {
        const x = request("GET", url + "?consh-ls");
        if (x && x.status === 200) {
          const type = x.getResponseHeader("Content-Type") || "";
          if (type.startsWith("application/json")) {
            for (const n of JSON.parse(new TextDecoder().decode(bytesOf(x.responseText)))) names.add(n.replace(/\/$/, ""));
          } else if (type.startsWith("text/html")) {
            // The listing of a directory: the links to names in it (an index.html gives none)
            const html = new TextDecoder().decode(bytesOf(x.responseText));
            if (/<title>\s*Directory listing for/i.test(html)) {
              for (const m of html.matchAll(/<a href="([^"?#]+)"/g)) {
                const n = decodeURIComponent(m[1]).replace(/\/$/, "");
                if (n !== "" && !n.includes("/") && n !== "." && n !== "..") names.add(n);
              }
            }
          }
        }
      }
      return [...names].sort();
    }

    // Open files: fd -> {entry, pos, mode}; an entry of mem is shared with the files open for it,
    // a file of the server read for reading is its own.
    const fds = new Map();
    let nextFd = 3;
    function add(f) {
      const fd = nextFd++;
      fds.set(fd, f);
      return fd;
    }

    const fs = {
      open(path, mode) {
        const p = normalize(path);
        if (p === null) return EINVAL;
        let m = mem.get(p);
        if (m && m.null) return add({ entry: m, pos: 0, mode });
        if (m && m.dir) return EISDIR;
        if (!m) {
          const pk = kind(parentOf(p));
          if (pk < 0) return pk;
          if (pk !== 1) return ENOTDIR;
        }
        if (mode === "r") {
          if (m) return add({ entry: m, pos: 0, mode });
          const s = server(p, true);
          if (s.kind < 0) return s.kind;
          if (s.kind === 1) return EISDIR;
          return add({ entry: { data: s.data }, pos: 0, mode });
        }
        // Writing: a file of memory, over a file of the server of the same name
        if (!m) {
          const s = mode === "a" ? server(p, true) : { kind: under(p) ? server(p, false).kind : ENOENT };
          if (s.kind === 1) return EISDIR;
          if (s.kind === EIO) return EIO;
          m = { data: s.kind === 2 && s.data ? s.data : new Uint8Array(0) };
          mem.set(p, m);
        } else if (mode === "w") {
          m.data = new Uint8Array(0);
        }
        return add({ entry: m, pos: 0, mode });
      },
      read(fd, len) {
        const f = fds.get(fd);
        if (!f) return EBADF;
        if (f.entry.null) return new Uint8Array(0);
        const r = f.entry.data.slice(f.pos, f.pos + len);
        f.pos += r.length;
        return r;
      },
      write(fd, bytes) {
        const f = fds.get(fd);
        if (!f) return EBADF;
        if (f.entry.null) return bytes.length;
        if (f.mode === "r") return EBADF;
        const e = f.entry;
        const at = f.mode === "a" ? e.data.length : f.pos;
        const end = at + bytes.length;
        if (end > e.data.length) {
          const d = new Uint8Array(end);
          d.set(e.data);
          e.data = d;
        }
        e.data.set(bytes, at);
        f.pos = end;
        return bytes.length;
      },
      close(fd) {
        return fds.delete(fd) ? 0 : EBADF;
      },
      // Another fd for the same open file (for a process that gets it, hostProc)
      dup(fd) {
        const f = fds.get(fd);
        return f ? add(f) : EBADF;
      },
      stat(path) {
        const p = normalize(path);
        return p === null ? EINVAL : kind(p);
      },
      realpath(path) {
        const p = normalize(path);
        if (p === null) return EINVAL;
        const k = kind(p);
        return k < 0 ? k : p;
      },
      readdir(path) {
        const p = normalize(path);
        return p === null ? EINVAL : list(p);
      },
      // For the builtin commands and the page: the bytes of a file (or minus an errno), a file put
      // in memory (0, or minus an errno)
      readFile(path) {
        const fd = fs.open(path, "r");
        if (fd < 0) return fd;
        const data = fds.get(fd).entry.data || new Uint8Array(0);
        fs.close(fd);
        return data;
      },
      writeFile(path, data) {
        const fd = fs.open(path, "w");
        if (fd < 0) return fd;
        fs.write(fd, data);
        return fs.close(fd);
      },
    };
    return fs;
  }

  return { create, normalize };
})();
