#!/usr/bin/env bash
# Build the pug-wasm crate for the browser and stage it into ./web/pkg.
set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown --features wasm -p pug-wasm

mkdir -p web/pkg
wasm-bindgen \
  --target web \
  --out-dir web/pkg \
  --out-name pug_wasm \
  target/wasm32-unknown-unknown/release/pug_wasm.wasm

# Report sizes
echo
ls -la web/pkg/*.wasm web/pkg/*.js | awk '{print $5, $9}'
