#!/usr/bin/env bash
# Build the toon-render-wasm crate for the browser and stage it into ./web/pkg.
set -euo pipefail

cd "$(dirname "$0")"

cargo build --release --target wasm32-unknown-unknown --features wasm -p toon-render-wasm

mkdir -p web/pkg
wasm-bindgen \
  --target web \
  --out-dir web/pkg \
  --out-name toon_render_wasm \
  target/wasm32-unknown-unknown/release/toon_render_wasm.wasm

# Report sizes
echo
ls -la web/pkg/*.wasm web/pkg/*.js | awk '{print $5, $9}'
