#!/usr/bin/env bash
# Build DBGo: the React frontend first, then the Tauri binary.
#
# The GUI has to be built with the `custom-protocol` feature, otherwise the
# window tries to load the UI from the Vite dev server (http://localhost:1420)
# and fails with "Could not connect to localhost" when no dev server is up.
#
#   ./build.sh           # release -> target/release/db-go
#   ./build.sh --debug   # debug   -> target/debug/db-go
#
# Anything else is forwarded to cargo, e.g.:
#   ./build.sh --locked
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

profile=release
cargo_args=(--release)
for arg in "$@"; do
  case "$arg" in
    --debug)
      profile=debug
      ;;
    --release)
      ;;
    -h|--help)
      awk 'NR>1 && /^#/ {sub(/^# ?/, ""); print; next} NR>1 {exit}' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      cargo_args+=("$arg")
      ;;
  esac
done
if [[ "$profile" == "debug" ]]; then
  cargo_args=()
fi

if [[ ! -d node_modules ]]; then
  echo "==> installing frontend dependencies"
  pnpm install
fi

echo "==> building frontend"
pnpm build

echo "==> building db-go ($profile)"
cargo build -p db-go --features custom-protocol ${cargo_args[@]+"${cargo_args[@]}"}

binary="$here/target/$profile/db-go"
if [[ ! -x "$binary" ]]; then
  echo "Build finished but $binary is missing." >&2
  exit 1
fi

# Guard against the exact bug that produces "Could not connect to localhost":
# a binary compiled without embedded frontend assets. Tauri stores the built
# asset paths (e.g. /assets/index-<hash>.js) in the embedded asset map, and
# they are absent when the UI is only loaded from the dev server.
asset_marker=""
asset_file="$(find "$here/dist/assets" -type f -name '*.js' -print -quit 2>/dev/null || true)"
[[ -n "$asset_file" ]] && asset_marker="/assets/$(basename "$asset_file")"

if command -v strings >/dev/null 2>&1; then
  if [[ -n "$asset_marker" ]]; then
    embedded=$(strings "$binary" | grep -cF "$asset_marker" || true)
  else
    embedded=$(strings "$binary" | grep -c "/assets/" || true)
  fi
  if [[ "${embedded:-0}" == "0" ]]; then
    echo
    echo "warning: $binary has no embedded frontend assets." >&2
    echo "         It will fail with 'Could not connect to localhost' unless the" >&2
    echo "         Vite dev server is running. Make sure 'pnpm build' succeeded" >&2
    echo "         and the binary was built with --features custom-protocol." >&2
  fi
fi

echo
echo "app built: $binary"
echo "install it:  ./install.sh"
