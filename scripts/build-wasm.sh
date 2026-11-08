#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-emscripten --target-dir target
mkdir -p frontend/public/wasm
cp target/wasm32-unknown-emscripten/release/deps/jpreprocess_vs_openjtalk.{js,wasm,data} frontend/public/wasm/
