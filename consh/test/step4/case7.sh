# スクリプトの最後の文の $( ) も、最後まで動く
z=$(sh -c 'echo to-stderr 1>&2')
