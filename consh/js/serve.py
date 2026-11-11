#!/usr/bin/env python3
"""Serve consh/js/web for ConSh in a browser, as python3 -m http.server does, and also:

* the entries of a directory for ?consh-ls, as JSON (the names of directories end with /), even when
  the directory has an index.html (ls of ConSh, web/serverfs.js);
* Cache-Control: no-cache, so that the browser checks a file with the server each time it is read
  (304 when it has not changed), and a file changed on the disk is seen at once;
* Cross-Origin-Opener-Policy and Cross-Origin-Embedder-Policy, so that the page is cross-origin isolated
  and can run the VM in a Worker (web/consh.js).
* Content-Security-Policy without 'unsafe-eval' (CSP_NO_EVAL), unless web/config.json turns on js or the SJS of
  the host ({"CONSH_ENABLE_JS": "js"} or "sjs"): then the browser stops eval and Function in the page and in the
  Worker, which nothing needs when they are off (consh/SAFEJS-PLAN-ja.md 5). The page sets the same policy
  itself (web/consh.js), for other servers.

It does not start, but says to run consh/js/build.sh, when a file of web/ that build.sh makes is
missing or older than its sources (conma.js, conma.wasm, consh.module.se, frontend.module.se, whose sources include
the frontend in bin/), as after git pull.

Usage: consh/js/serve.py [port]   (8765 by default; only on 127.0.0.1)
"""
import functools
import http.server
import json
import os
import sys
import urllib.parse

WEB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "web")


# The same as CSP_NO_EVAL of web/consh.js: scripts only from this server, WebAssembly allowed, no eval or Function
CSP_NO_EVAL = "script-src 'self' 'wasm-unsafe-eval'; object-src 'none'; base-uri 'none'"


def host_js_enabled():
    """Whether web/config.json turns on js or the SJS of the host (read at each request, as the page reads it)."""
    try:
        with open(os.path.join(WEB, "config.json"), encoding="utf-8") as f:
            config = json.load(f)
    except (OSError, ValueError):
        return False
    return isinstance(config, dict) and config.get("CONSH_ENABLE_JS") in ("js", "sjs")


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        if not host_js_enabled():
            self.send_header("Content-Security-Policy", CSP_NO_EVAL)
        # Cross-origin isolated, so that the page can give the Worker of the VM a SharedArrayBuffer
        # (web/consh.js); then a resource of another origin needs CORS or Cross-Origin-Resource-Policy
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def do_GET(self):
        url = urllib.parse.urlsplit(self.path)
        # A page is always sent in full (200, not 304), so that the browser takes its Content-Security-Policy
        # as it is now: a 304 keeps the headers of the copy in the cache, so turning js on or off in config.json
        # would not change the policy of a page read before (the scripts and the Worker are read with ?Date.now())
        if url.path.endswith("/") or url.path.endswith(".html"):
            del self.headers["If-Modified-Since"]
            del self.headers["If-None-Match"]
        if url.query == "consh-ls":
            path = self.translate_path(url.path)
            if not os.path.isdir(path):
                self.send_error(404)
                return
            names = sorted(n + "/" if os.path.isdir(os.path.join(path, n)) else n for n in os.listdir(path))
            body = json.dumps(names).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        super().do_GET()


JS = os.path.dirname(WEB)
ROOT = os.path.dirname(os.path.dirname(JS))


def newest(*dirs, suffixes=(".rs", ".se", ".toml")):
    """The newest time of the files with the suffixes, and of the folders (a file added, removed or
    moved changes the time of its folder, as build.sh sees it), under dirs."""
    t = 0.0
    for d in dirs:
        for top, subdirs, files in os.walk(d):
            subdirs[:] = [s for s in subdirs if s != "target"]
            t = max(t, os.path.getmtime(top))
            for f in files:
                if f.endswith(suffixes):
                    t = max(t, os.path.getmtime(os.path.join(top, f)))
    return t


def stale():
    """The files of web/ that consh/js/build.sh makes and that are missing or older than their
    sources (git pull does not change them, since they are not in the repository)."""
    found = []
    built = os.path.join(WEB, "conma.js")
    try:
        with open(built, "rb") as a, open(os.path.join(JS, "conma.js"), "rb") as b:
            if a.read() != b.read():
                found.append(built)
    except FileNotFoundError:
        found.append(built)
    sources = {
        "conma.wasm": newest(os.path.join(ROOT, "vm", "src"), os.path.join(JS, "wasm")),
        # the sources of ConSh, and the frontend that makes the module (bin/includer and the others)
        "consh.module.se": max(
            newest(
                os.path.join(ROOT, "include"),
                os.path.join(ROOT, "consh", "include"),
                suffixes=(".se",),
            ),
            newest(os.path.join(ROOT, "bin"), suffixes=("",)),
        ),
    }
    # the frontend of ConMa, copied by build.sh (web/consh-core.js makes the modules of SJS with it)
    sources["frontend.module.se"] = os.path.getmtime(os.path.join(ROOT, "vm", "frontend.module.se"))
    for name, source_time in sources.items():
        path = os.path.join(WEB, name)
        if not os.path.exists(path) or os.path.getmtime(path) < source_time:
            found.append(path)
    return found


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    old = stale()
    if old:
        for path in old:
            print(f"serve.py: {os.path.relpath(path)} is missing or older than its sources", file=sys.stderr)
        print("serve.py: run consh/js/build.sh first", file=sys.stderr)
        sys.exit(1)
    handler = functools.partial(Handler, directory=WEB)
    with http.server.ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:
        print(f"Serving {WEB} on http://localhost:{port}/", flush=True)
        httpd.serve_forever()


if __name__ == "__main__":
    main()
