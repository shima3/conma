# $$ はシェルのプロセスID。$( ) の中でも同じ
if [ "$$" -gt 0 ]; then echo positive; fi
if [ "$$" = "$(echo $$)" ]; then echo same; fi
