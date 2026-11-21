# case6.con.js の標準入力：30 文字の行を 40 個、それぞれ行の途中で 2 つに分け、間を空けて送る（M2 の PR shima3/conma#21 の
# レビューの形。乱数の種を決めてあるので、分ける所と間は毎回同じ。前の版 290085d では、この入力で行が壊れた）
import sys, time, random
random.seed(4)
for i in range(40):
    line = b"x" * 30 + b"\n"
    k = random.randint(1, 29)
    sys.stdout.buffer.write(line[:k]); sys.stdout.flush(); time.sleep(random.random() * 0.01)
    sys.stdout.buffer.write(line[k:]); sys.stdout.flush(); time.sleep(random.random() * 0.01)
