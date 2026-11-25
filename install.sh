#!/usr/bin/env bash
# Install DBGo for the current user: binary, icons and a desktop entry so the
# app shows up in the GNOME/KDE application menu (the "start" button) with the
# right logo, and in docks/taskbars (WM class "db-go").
#
# Build first:
#   ./build.sh
# Then:
#   ./install.sh
#
# Options:
#   PREFIX=/usr/local ./install.sh   # install system-wide (needs sudo)
#   ./install.sh --no-cli            # skip the `dbgo` command-line tool
#   ./install.sh --uninstall         # remove everything this script installed
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$here"

app="db-go"
product="DBGo"

prefix="${PREFIX:-$HOME/.local}"
if [[ -z "${PREFIX:-}" && -n "${XDG_DATA_HOME:-}" ]]; then
  data_dir="$XDG_DATA_HOME"
else
  data_dir="$prefix/share"
fi
bin_dir="$prefix/bin"
lib_dir="$prefix/lib/db-go"
apps_dir="$data_dir/applications"
icons_dir="$data_dir/icons/hicolor"

want_cli=1
uninstall=0
for arg in "$@"; do
  case "$arg" in
    --no-cli) want_cli=0 ;;
    --uninstall) uninstall=1 ;;
    -h|--help)
      awk 'NR>1 && /^#/ {sub(/^# ?/, ""); print; next} NR>1 {exit}' "${BASH_SOURCE[0]}"
      exit 0
      ;;
    *)
      echo "unknown option: $arg" >&2
      exit 2
      ;;
  esac
done

refresh_icon_cache() {
  local tool
  for tool in gtk4-update-icon-cache gtk-update-icon-cache; do
    if command -v "$tool" >/dev/null 2>&1; then
      "$tool" -f -t "$icons_dir" >/dev/null 2>&1
      return 0
    fi
  done
  return 0
}

icon_targets=(
  "32x32:$root/src-tauri/icons/32x32.png"
  "64x64:$root/src-tauri/icons/64x64.png"
  "128x128:$root/src-tauri/icons/128x128.png"
  "256x256:$root/src-tauri/icons/128x128@2x.png"
  "512x512:$root/src-tauri/icons/icon.png"
)

if [[ "$uninstall" == "1" ]]; then
  rm -f "$bin_dir/$app" "$bin_dir/dbgo"
  rm -f "$lib_dir/$app"
  rmdir "$lib_dir" 2>/dev/null || true
  rm -f "$apps_dir/db-go.desktop"
  for entry in "${icon_targets[@]}"; do
    rm -f "$icons_dir/${entry%%:*}/apps/db-go.png"
  done
  rm -f "$icons_dir/scalable/apps/db-go.svg"

  command -v update-desktop-database >/dev/null 2>&1 && \
    update-desktop-database "$apps_dir" >/dev/null 2>&1 || true
  refresh_icon_cache || true

  echo "$product removed from $prefix"
  exit 0
fi

binary="$root/target/release/$app"
if [[ ! -x "$binary" ]]; then
  echo "Release binary not found: $binary" >&2
  echo "Build it first with: ./build.sh" >&2
  exit 1
fi

# Refuse to install a build that will open an empty window: no embedded
# frontend assets means it would ask the (usually absent) Vite dev server.
asset_marker=""
asset_file="$(find "$root/dist/assets" -type f -name '*.js' -print -quit 2>/dev/null || true)"
[[ -n "$asset_file" ]] && asset_marker="/assets/$(basename "$asset_file")"

if command -v strings >/dev/null 2>&1; then
  if [[ -n "$asset_marker" ]]; then
    embedded=$(strings "$binary" | grep -cF "$asset_marker" || true)
  else
    embedded=$(strings "$binary" | grep -c "/assets/" || true)
  fi
  if [[ "${embedded:-0}" == "0" ]]; then
    echo "Refusing to install $binary:" >&2
    echo "  it has no embedded frontend assets and will fail with" >&2
    echo "  'Could not connect to localhost'." >&2
    echo "  Rebuild with: ./build.sh" >&2
    exit 1
  fi
fi

mkdir -p "$bin_dir" "$lib_dir" "$apps_dir"

install -Dm755 "$binary" "$lib_dir/$app"
ln -sfn "$lib_dir/$app" "$bin_dir/$app"

if [[ "$want_cli" == "1" && -x "$root/target/release/dbgo" ]]; then
  install -Dm755 "$root/target/release/dbgo" "$bin_dir/dbgo"
fi

# PNG icons for menus, docks and file managers.
for entry in "${icon_targets[@]}"; do
  size="${entry%%:*}"
  source="${entry#*:}"
  [[ -f "$source" ]] || continue
  install -Dm644 "$source" "$icons_dir/$size/apps/db-go.png"
done

# Scalable icon (crisp at any size) when the source SVG is available.
if [[ -f "$root/assets/dbgo-icon.svg" ]]; then
  install -Dm644 "$root/assets/dbgo-icon.svg" "$icons_dir/scalable/apps/db-go.svg"
fi

# Desktop entry, with Exec pointing at this prefix.
sed "s#^Exec=.*#Exec=$bin_dir/$app %f#" \
  "$root/packaging/db-go.desktop" > "$apps_dir/db-go.desktop"
chmod 644 "$apps_dir/db-go.desktop"

if command -v desktop-file-validate >/dev/null 2>&1; then
  desktop-file-validate "$apps_dir/db-go.desktop" || true
fi

command -v update-desktop-database >/dev/null 2>&1 && \
  update-desktop-database "$apps_dir" >/dev/null 2>&1 || true

refresh_icon_cache || true

echo "$product installed:"
echo "  binary:  $bin_dir/$app"
echo "  icons:   $icons_dir/*/apps/db-go.*"
echo "  desktop: $apps_dir/db-go.desktop"
echo
if ! command -v "$app" >/dev/null 2>&1; then
  echo "Add $bin_dir to PATH, then launch it from the application menu or with:"
  echo "  $app"
else
  echo "Launch it from the application menu (search \"$product\") or with: $app"
fi
echo "Pin it to the dock by right-clicking its icon in the dock or app grid."
