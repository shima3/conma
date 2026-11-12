#!/bin/bash
# ConMa の VM を起動する。引数は vproc_run.py と同じ（[--trace] [--main] <module_file> <func_name | arg ...>）。
# 環境変数 CONMA_VM が rust なら Rust 版（vm/target/release/conma）、transpiled なら ConMa から Rust への変換器
# （vm/conma2rust）で変換したコードを組み込んだ Rust 版
# （vm/conma2rust/build.sh で作る vm/target-transpiled/release/conma）、wasm なら wasm32 版を JavaScriptCore で動かす
# （consh/js/vm-wasm.sh。consh/js/build.sh で作る）、それ以外（既定）は Python 版（vproc_run.py。高速版 ConMa の
# モジュールは bin/concise で簡潔版に直してから動かす）
dir="$(cd "${0%/*}"; pwd)"
case "${CONMA_VM:-python}" in
  rust) exec "$dir/../vm/target/release/conma" "$@" ;;
  wasm) exec "$dir/../consh/js/vm-wasm.sh" "$@" ;;
  transpiled) exec "$dir/../vm/target-transpiled/release/conma" "$@" ;;
  *)
    # Python VM は簡潔版の VM なので、高速版 ConMa のモジュール（(Uses "機能" ...) のあるもの）は、bin/concise で
    # 簡潔版に直してから動かす（docs/FAST-PLAN-ja.md の 3.4）。直せない機能なら、そのまま動かす（VM がエラーにする）
    args=("$@")
    for ((i = 0; i < ${#args[@]}; i++)); do
      case "${args[$i]}" in
        -t|--trace|-m|--main|--) ;;
        *)
          module="${args[$i]}"
          if [ -f "$module" ] && grep -q '(Uses "' "$module"; then
            concise="$(mktemp "${TMPDIR:-/tmp}/conma-concise.XXXXXX")" || exit 1
            if "$dir/../bin/concise" "$module" > "$concise" 2>/dev/null; then
              args[$i]="$concise"
              python3 "$dir/vproc_run.py" "${args[@]}"
              status=$?
              rm -f "$concise"
              exit $status
            fi
            rm -f "$concise"
          fi
          break ;;
      esac
    done
    exec python3 "$dir/vproc_run.py" "$@" ;;
esac
