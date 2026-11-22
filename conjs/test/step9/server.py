# conjs/test/step9 の fetch のテストのための、小さな HTTP サーバ（127.0.0.1。docs/dev/PLATFORM-PLAN-ja.md の 4.4 節、段階 P5）。
#   python3 server.py PORTFILE
# OS が選んだポートで待ち、その番号を PORTFILE に書く。標準入力が閉じたら終わる（テストが終わればサーバも終わる）。
#   GET /hello       200、本体 "hello from server"
#   GET /json        200、本体 {"a": [1, 2], "b": "x"}
#   POST /echo       200、本体 "METHOD 本体 X-Test ヘッダの値"
#   HEAD /hello      200、本体なし（GET /hello と同じヘッダ）
#   ほか             404、本体 "not found"
import http.server, socketserver, sys, threading

class Handler(http.server.BaseHTTPRequestHandler):
    def reply(self, code, body):
        data = body.encode()
        self.send_response(code)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        if self.path == "/hello":
            self.reply(200, "hello from server")
        elif self.path == "/json":
            self.reply(200, '{"a": [1, 2], "b": "x"}')
        else:
            self.reply(404, "not found")

    def do_POST(self):
        n = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(n).decode()
        if self.path == "/echo":
            self.reply(200, "%s %s %s" % (self.command, body, self.headers.get("X-Test")))
        else:
            self.reply(404, "not found")

    do_PUT = do_POST

    def do_HEAD(self):
        code = 200 if self.path == "/hello" else 404
        self.send_response(code)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", "17" if code == 200 else "9")
        self.end_headers()

    def log_message(self, *args):
        pass

class Server(socketserver.ThreadingMixIn, http.server.HTTPServer):
    daemon_threads = True

    # HTTPServer.server_bind は socket.getfqdn で 127.0.0.1 の名前を引き、GitHub Actions の macos-latest では、
    # run_case.sh が待つ 10 秒を越えることがあった（2026-11-22）。名前は使わないので引かない
    def server_bind(self):
        socketserver.TCPServer.server_bind(self)
        self.server_name, self.server_port = self.server_address[:2]

server = Server(("127.0.0.1", 0), Handler)
with open(sys.argv[1], "w") as f:
    f.write(str(server.server_address[1]))
threading.Thread(target=server.serve_forever, daemon=True).start()
sys.stdin.read()
