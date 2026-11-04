# パイプラインの中から外のループへの break はエラー
for x in a b; do if true; then break; fi | cat; done
