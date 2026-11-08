#!/usr/bin/env python3
"""Serve consh/js/web for ConSh in a browser, as python3 -m http.server does, and also:

* the entries of a directory for ?consh-ls, as JSON (the names of directories end with /), even when
  the directory has an index.html (ls of ConSh, web/serverfs.js);
* Cache-Control: no-cache, so that the browser checks a file with the server each time it is read
  (304 when it has not changed), and a file changed on the disk is seen at once.

Usage: consh/js/serve.py [port]   (8765 by default; only on 127.0.0.1)
"""
import functools
import http.server
import json
import os
import sys
import urllib.parse

WEB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "web")


class Handler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-cache")
        super().end_headers()

    def do_GET(self):
        url = urllib.parse.urlsplit(self.path)
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


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8765
    handler = functools.partial(Handler, directory=WEB)
    with http.server.ThreadingHTTPServer(("127.0.0.1", port), handler) as httpd:
        print(f"Serving {WEB} on http://localhost:{port}/", flush=True)
        httpd.serve_forever()


if __name__ == "__main__":
    main()
