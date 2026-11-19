#!/bin/bash
# ConJS のテストの 1 つのステップ（docs/dev/PLATFORM-PLAN-ja.md の 4.2 節）。2026-11-20 から、言語キットのテストの型
# bin/lang-test（4.3 節）を呼ぶだけにした。ステップのディレクトリの case*.con.js を変換して動かし、出力と終了ステータスを
# case*.log と比べる。VM は CONMA_VM（なし：Rust 版、wasm、python）
#   run_case.sh ステップのディレクトリ
here="$(cd "$(dirname "$0")"; pwd)"
exec "$here/../../bin/lang-test" "$here/.." "$1"
