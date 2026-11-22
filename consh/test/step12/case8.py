# 端末（疑似端末）で Ctrl-C を押したとき、前景の外部コマンドは止まり、& で起動した外部コマンドは止まらないか。
# consh を疑似端末の中で起動し、入力を送って、出力を表示する（改行の \r を除く）。
# 入力は、プロンプトが出てから送り、Ctrl-C は前景の sleep が動き出してから送る。Python 版の VM は起動に数秒かかり、
# __OS_trap_interrupt__ の前の Ctrl-C は VM を止めるので、決まった秒数だけ待つのでは足りない
import os, pty, select, subprocess, sys, time
consh = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../bin/consh")
env = dict(os.environ, CONSH_NO_RLWRAP="1")
pid, fd = pty.fork()
if pid == 0:
    os.execve(consh, ["consh"], env)
out = b""
LIMIT = 120     # 待つ秒数の上限（Python 版の VM でも十分なだけ）
def read(timeout):
    global out
    r, _, _ = select.select([fd], [], [], timeout)
    if r:
        try:
            data = os.read(fd, 4096)
        except OSError:
            data = b""
        if not data:
            return False
        out += data
    return True
def drain(seconds):
    end = time.time() + seconds
    while time.time() < end and read(0.1):
        pass
def until(done):
    # done() が真になるまで出力を読む（上限 LIMIT 秒）
    end = time.time() + LIMIT
    while not done() and time.time() < end and read(0.1):
        pass
mark = 0        # 最後に入力を送ったときの出力の長さ
def prompt():
    # 最後の入力の後で、プロンプト（"$ "）が出て、入力を待つまで
    until(lambda: len(out) > mark and out.endswith(b"$ "))
def sleeping():
    # pid の子孫に sleep 10 のプロセスがあるか
    ps = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,command="], capture_output=True, text=True).stdout
    children, commands = {}, {}
    for line in ps.splitlines():
        p, pp, command = (line.split(None, 2) + [""])[:3]
        children.setdefault(int(pp), []).append(int(p))
        commands[int(p)] = command
    todo = [pid]
    while todo:
        for c in children.get(todo.pop(), []):
            args = commands[c].split()
            if [os.path.basename(a) for a in args[:1]] + args[1:] == ["sleep", "10"]:   # Rust 版は /bin/sleep 10
                return True
            todo.append(c)
    return False
def send(data):
    global mark
    mark = len(out)
    os.write(fd, data)
prompt()
send(b"sh -c 'sleep 6; echo bg-done' &\n")
prompt()
send(b"sleep 10\n")
# 端末に返した "sleep 10" を読んでから（Ctrl-C は、まだ読んでいない出力を捨てる）
until(lambda: out.endswith(b"sleep 10\r\n"))
until(sleeping)
send(b"\x03")
prompt()
send(b"echo status $?\n")
prompt()
send(b"wait\n")
prompt()
send(b"exit\n")
drain(1)
os.waitpid(pid, 0)
sys.stdout.write(out.decode().replace("\r", ""))
