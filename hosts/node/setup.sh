#!/bin/sh
# Builds the module cascade-bridge.mjs loads, into hosts/node/pkg, with the
# tools it needs: the wasm32 target, wasm-bindgen-cli at exactly the version of
# the wasm-bindgen crate in Cargo.lock, since the two must agree, and wasm-opt.
# Every tool is installed under the target directory, never into $CARGO_HOME.
set -eu
cd "$(dirname "$0")/../.."
target="${CARGO_TARGET_DIR:-target}"
binaryen=version_133

rustup target add wasm32-unknown-unknown

bindgen=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
bindgen_root="$target/wasm-bindgen-$bindgen"
if [ ! -x "$bindgen_root/bin/wasm-bindgen" ] && [ ! -x "$bindgen_root/bin/wasm-bindgen.exe" ]; then
  cargo install --locked wasm-bindgen-cli --version "=$bindgen" --root "$bindgen_root"
fi

exe=
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) platform=x86_64-linux
    sha256=2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489 ;;
  Linux-aarch64) platform=aarch64-linux
    sha256=89c07ea56faf38d0fbecf36ca8ec0721756716185f265b568e133d427f299bf8 ;;
  Darwin-x86_64) platform=x86_64-macos
    sha256=13a9b90be775c6389ce3d1f879cb8627bea56708ba8c122983941d53a8199b95 ;;
  Darwin-arm64) platform=arm64-macos
    sha256=ad66da82ac13f163e424b1643f16c6dfcccc98b5966296b43e52d3cab04f84a8 ;;
  MINGW*-x86_64 | MSYS*-x86_64 | CYGWIN*-x86_64) platform=x86_64-windows exe=.exe
    sha256=17a2cbeac6b5693c5fbafab3838d3c65fd9c1eb38b05f5baec6c657e8c84995b ;;
  MINGW*-aarch64 | MSYS*-aarch64 | CYGWIN*-aarch64) platform=arm64-windows exe=.exe
    sha256=492a8e1847a0be1554bb9a7f384227981d60bc013aedc02d8ba1372c3943178c ;;
  *) echo "setup: no binaryen release for $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

opt="$target/binaryen-$binaryen/bin/wasm-opt$exe"
if [ ! -x "$opt" ]; then
  mkdir -p "$target"
  tarball="$target/binaryen-$binaryen-$platform.tar.gz"
  curl -fsSL -o "$tarball" \
    "https://github.com/WebAssembly/binaryen/releases/download/$binaryen/binaryen-$binaryen-$platform.tar.gz"
  if command -v sha256sum >/dev/null 2>&1; then
    got=$(sha256sum "$tarball" | cut -d' ' -f1)
  else
    got=$(shasum -a 256 "$tarball" | cut -d' ' -f1)
  fi
  if [ "$got" != "$sha256" ]; then
    rm -f "$tarball"
    echo "setup: binaryen-$binaryen-$platform.tar.gz has sha256 $got, not $sha256" >&2
    exit 1
  fi
  tar -xzf "$tarball" -C "$target"
  rm -f "$tarball"
fi

cargo build --release --locked -p cascade-bridge-wasm --target wasm32-unknown-unknown
built="$target/wasm32-unknown-unknown/release/cascade_bridge_wasm.wasm"
pkg=hosts/node/pkg
loaded=$pkg/cascade_bridge_wasm_bg.wasm
# The module is rebuilt only when cargo or this script changed since it was
# written, so a second run on an unchanged tree does nothing.
if [ -f "$loaded" ] && [ -f "$pkg/cascade_bridge_wasm.js" ] &&
  [ ! "$built" -nt "$loaded" ] && [ ! hosts/node/setup.sh -nt "$loaded" ]; then
  exit 0
fi
# Built beside pkg and moved in only once wasm-opt succeeds: a run that stops
# partway must leave nothing the check above takes for finished.
staged=$pkg.partial
rm -rf "$staged"
"$bindgen_root/bin/wasm-bindgen" --target nodejs --out-dir "$staged" "$built"
"$opt" -Oz --strip-debug --strip-producers --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
  --enable-mutable-globals --enable-reference-types --enable-multivalue \
  "$staged/cascade_bridge_wasm_bg.wasm" -o "$staged/cascade_bridge_wasm_bg.wasm"
rm -rf "$pkg"
mv "$staged" "$pkg"
