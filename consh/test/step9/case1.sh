# 引数のバイト数は、コマンド名を含む各引数の UTF-8 のバイト数 + 1（NUL）の合計。ARG_MAX 以下なら起動する
# printf(7) + %s(3) + abc(4) = 14
ARG_MAX=14 printf %s abc
echo
# あ は 3 バイト：printf(7) + %s(3) + あ(4) = 14
ARG_MAX=14 printf %s あ
echo
# 15 バイトで 14 を超える
ARG_MAX=14 printf %s abcd
echo not reached
