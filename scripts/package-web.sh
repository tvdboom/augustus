#!/usr/bin/env bash
set -euo pipefail

repository="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
wasm_bindgen_version="$(tr -d '[:space:]' < "$repository/scripts/wasm-bindgen-version.txt")"

if [[ ! -f "$repository/assets-runtime/.augustus-assets" ]]; then
  echo "Runtime assets are missing. Install KTX-Software 4.x and run 'just assets' before packaging." >&2
  exit 1
fi

source "$repository/scripts/common.sh"
prepare_package_output
stage="$output_root/augustus-html"
archive="$output_root/augustus-html.zip"
rm -rf -- "$stage"
mkdir -p "$stage"

if [[ "${SKIP_BUILD:-0}" != "1" ]]; then
  cargo build --manifest-path "$repository/Cargo.toml" --profile wasm-release \
    --target wasm32-unknown-unknown --bin augustus -j12
fi

installed_version="$(wasm-bindgen --version | sed 's/^wasm-bindgen //')"
if [[ "$installed_version" != "$wasm_bindgen_version" ]]; then
  echo "wasm-bindgen-cli $wasm_bindgen_version is required; found $installed_version" >&2
  exit 1
fi

wasm-bindgen "$repository/target/wasm32-unknown-unknown/wasm-release/augustus.wasm" \
  --target web --out-dir "$stage" --out-name augustus --no-typescript
cp "$repository/web/index.html" "$repository/LICENSE" "$repository/README.md" "$stage/"
mkdir -p "$stage/assets/images"
cp -R "$repository/assets/images/icons" "$stage/assets/images/icons"
cp -R "$repository/assets-runtime" "$stage/assets-runtime"

rm -f -- "$archive"
(cd "$stage" && zip -qr "$archive" .)
echo "Created $archive"
