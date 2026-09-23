#!/bin/sh
# Builds the module cascade-bridge.mjs loads, into hosts/node/pkg, with the
# tools it needs: the wasm32 target, wasm-bindgen-cli at exactly the version of
# the wasm-bindgen crate in Cargo.lock, since the two must agree, and wasm-opt.
set -eu
cd "$(dirname "$0")/../.."
target="${CARGO_TARGET_DIR:-target}"
binaryen=version_133

rustup target add wasm32-unknown-unknown

bindgen=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
if [ "$(wasm-bindgen --version 2>/dev/null)" != "wasm-bindgen $bindgen" ]; then
  cargo install --locked --force wasm-bindgen-cli --version "=$bindgen"
fi

opt="$target/binaryen-$binaryen/bin/wasm-opt"
if [ ! -x "$opt" ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) platform=x86_64-linux ;;
    Linux-aarch64) platform=aarch64-linux ;;
    Darwin-x86_64) platform=x86_64-macos ;;
    Darwin-arm64) platform=arm64-macos ;;
    *) echo "setup: no binaryen release for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
  esac
  mkdir -p "$target"
  curl -fsSL "https://github.com/WebAssembly/binaryen/releases/download/$binaryen/binaryen-$binaryen-$platform.tar.gz" \
    | tar -xz -C "$target"
fi

cargo build --release --locked -p cascade-bridge-wasm --target wasm32-unknown-unknown
wasm-bindgen --target nodejs --out-dir hosts/node/pkg \
  "$target/wasm32-unknown-unknown/release/cascade_bridge_wasm.wasm"
"$opt" -Oz --strip-debug --strip-producers --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
  --enable-mutable-globals --enable-reference-types --enable-multivalue \
  hosts/node/pkg/cascade_bridge_wasm_bg.wasm -o hosts/node/pkg/cascade_bridge_wasm_bg.wasm
