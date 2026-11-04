# 端末（疑似端末）で Ctrl-C を押したとき、前景の外部コマンドは止まり、& で起動した外部コマンドは止まらないか。
# consh を疑似端末の中で起動し、入力を送って、出力を表示する（改行の \r を除く）
import os, pty, select, sys, time
consh = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../bin/consh")
env = dict(os.environ, CONSH_NO_RLWRAP="1")
pid, fd = pty.fork()
if pid == 0:
    os.execve(consh, ["consh"], env)
out = b""
def drain(seconds):
    global out
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.1)
        if r:
            try:
                out += os.read(fd, 4096)
            except OSError:
                return
def send(data, seconds):
    os.write(fd, data)
    drain(seconds)
drain(3)
send(b"sh -c 'sleep 6; echo bg-done' &\n", 1)
send(b"sleep 10\n", 1)
send(b"\x03", 1)
send(b"echo status $?\n", 1)
send(b"wait\n", 6)
send(b"exit\n", 1)
os.waitpid(pid, 0)
sys.stdout.write(out.decode().replace("\r", ""))
