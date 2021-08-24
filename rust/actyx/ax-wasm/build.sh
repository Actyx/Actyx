#!/bin/bash -e
WASMOPT=./wasm-opt
WASMOPT_VERSION=101
WASMBINDGEN_VERSION=0.2.76
OUT=./pkg

echo "Running cargo build"
cargo build --release --target wasm32-unknown-unknown

if [ -d $OUT ]; then
  echo "Clearing output directory '$OUT'"
  rm -rf $OUT
fi

if ! [ -x "$(command -v wasm-bindgen)" ]; then
  echo "Installing wasm-bindgen-cli via cargo"
  cargo install wasm-bindgen-cli --version 0.2.76
fi

echo "Generating wasm-bindings"

# add supports for Weak References, see [1].
# TLDR: Structs passed from Rust to JS will be deallocated
# automatically, no need to call `.free` in JS.
#
# [1]: https://rustwasm.github.io/docs/wasm-bindgen/reference/weak-references.html
wasm-bindgen ../target/wasm32-unknown-unknown/release/ax_wasm.wasm \
  --out-dir $OUT \
  --target web \
  --typescript \
  --weak-refs
  # --reference-types TODO: wasm-opt crashes with that flag

echo "Generating package.json"
cat <<EOF >> $OUT/package.json
{
  "name": "ax-wasm",
  "version": "0.1.0",
  "files": [
    "ax_wasm_bg.wasm",
    "ax_wasm.js",
    "ax_wasm.d.ts"
  ],
  "module": "ax_wasm.js",
  "types": "ax_wasm.d.ts",
  "sideEffects": false
}
EOF


if [ ! -f $WASMOPT ]; then
  echo "Downloading wasm-opt"
  wget -qO- \
  https://github.com/WebAssembly/binaryen/releases/download/version_$WASMOPT_VERSION/binaryen-version_$WASMOPT_VERSION-x86_64-linux.tar.gz \
  | tar xfz - binaryen-version_$WASMOPT_VERSION/bin/wasm-opt -O >> $WASMOPT
  chmod +x $WASMOPT
fi

echo "Optimizing wasm bindings with default optimization (this might take some time)"
./wasm-opt $OUT/ax_wasm_bg.wasm -O -g --output $OUT/ax_wasm_bg.opt.wasm

echo "Find your wasm package in $OUT"
