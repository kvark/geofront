#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
TARGET="${CARGO_TARGET_DIR:-target}"
echo "building geofront wasm"
cargo build --release --target wasm32-unknown-unknown --lib
mkdir -p dist/pkg
wasm-bindgen --target web --no-typescript \
  --out-dir dist/pkg \
  "$TARGET/wasm32-unknown-unknown/release/geofront.wasm"
cp -f web/index.html dist/index.html
cp -f web/favicon.svg web/og.jpg web/x-banner.jpg dist/ 2>/dev/null || true
touch dist/.nojekyll
echo "web build -> dist"
