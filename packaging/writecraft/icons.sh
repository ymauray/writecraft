#!/usr/bin/env bash
# WriteCraft: regenerate the WriteCraft app icons from assets/writecraft-icon/writecraft.svg.
# (WordCraft's own icons stay in assets/app-icon, made by packaging/icons.sh.)
#
# Needs: resvg (brew install resvg). On macOS, iconutil also writes the .icns. The outputs are
# committed, so builds and packaging never need these tools.
#
#   packaging/writecraft/icons.sh
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIR="$ROOT/assets/writecraft-icon"
SVG="$DIR/writecraft.svg"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

command -v resvg >/dev/null || { echo "error: resvg not found (brew install resvg)" >&2; exit 1; }

# The artwork is a full-bleed 512 tile. Windows uses it as is; macOS icons sit on Apple's grid (an
# 824/1024 body with a transparent margin), so widen the viewBox to 512*1024/824.
MAC="$TMP/mac.svg"
sed 's/viewBox="0 0 512 512"/viewBox="-62.136 -62.136 636.272 636.272"/' "$SVG" >"$MAC"
grep -q 'viewBox="-62.136' "$MAC" || { echo "error: expected viewBox=\"0 0 512 512\" in $SVG" >&2; exit 1; }

render() {
  resvg -w "$2" -h "$2" "$1" "$3" </dev/null
}

render "$SVG" 1024 "$DIR/writecraft-1024.png"
# Runtime window/Dock icon: macOS uses the 512 on Apple's grid, the others the 256 tile.
render "$MAC" 512 "$DIR/writecraft-macos-512.png"
render "$SVG" 256 "$DIR/writecraft-256.png"

# Windows .ico.
ICO_PNGS=()
for s in 16 20 24 32 40 48 64 128 256; do
  render "$SVG" "$s" "$TMP/ico-$s.png"
  ICO_PNGS+=("$TMP/ico-$s.png")
done
(cd "$ROOT" && cargo run -q -p xtask -- ico "$DIR/writecraft.ico" "${ICO_PNGS[@]}")

# macOS .icns.
if command -v iconutil >/dev/null; then
  SET="$TMP/writecraft.iconset"
  mkdir -p "$SET"
  for s in 16 32 128 256 512; do
    render "$MAC" "$s" "$SET/icon_${s}x${s}.png"
    render "$MAC" $((s * 2)) "$SET/icon_${s}x${s}@2x.png"
  done
  iconutil -c icns -o "$DIR/writecraft.icns" "$SET"
else
  echo "warning: iconutil not found (macOS only); writecraft.icns not regenerated" >&2
fi
echo "icons written to $DIR"
