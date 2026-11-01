#!/usr/bin/env bash
# WriteCraft: build and sign the macOS app, on a drag-to-Applications DMG:
#
#   $DIST/writecraft-<version>-macos-<arch>.dmg      WriteCraft.app
#
# WriteCraft's own bundle (name, id io.github.ymauray.writecraft, icon), so it installs beside
# WordCraft. Adapted from WordCraft's packaging/macos/package.sh, which stays as upstream has it.
#
# Usage: packaging/writecraft/macos.sh [--arch universal|aarch64|x86_64] [--skip-build]
#
# Env:
#   WRITECRAFT_VERSION    version (default 0.0.0-dev); the release workflow sets it from the tag
#   CRAFT_I18N_DIR        craft-i18n checkout (translations); CRAFT_I18N_REQUIRED=1 makes it mandatory
#   MACOS_SIGN_IDENTITY   codesign identity. Default "-" = ad-hoc: runs on this Mac; on another Mac
#                         Gatekeeper asks to confirm (right-click › Open, or System Settings).
#   APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID   notarization, with a real identity (optional)
set -euo pipefail
# shellcheck source=../env.sh
. "$(dirname "${BASH_SOURCE[0]}")/../env.sh"
HERE="$ROOT/packaging/writecraft"

ARCH=universal
SKIP_BUILD=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --arch) ARCH="$2"; shift 2 ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    -h | --help) sed -n '2,17p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$ARCH" in
  universal) TARGETS=(aarch64-apple-darwin x86_64-apple-darwin) ;;
  aarch64) TARGETS=(aarch64-apple-darwin) ;;
  x86_64) TARGETS=(x86_64-apple-darwin) ;;
  *) echo "unknown --arch $ARCH" >&2; exit 2 ;;
esac

VERSION="${WRITECRAFT_VERSION:-0.0.0-dev}"
export WRITECRAFT_VERSION="$VERSION"
SHORT_VERSION="${VERSION%%-*}"
# Keep in sync with LSMinimumSystemVersion in Info.plist.in.
export MACOSX_DEPLOYMENT_TARGET=11.0
IDENTITY="${MACOS_SIGN_IDENTITY:--}"
WORK="$CARGO_TARGET_DIR/writecraft-macos"
APP="$WORK/WriteCraft.app"
DMG="$DIST/writecraft-$VERSION-macos-$ARCH.dmg"

NOTARIZE=0
if [[ "$IDENTITY" = "-" ]]; then
  warn "macOS: ad-hoc signing (no MACOS_SIGN_IDENTITY); not notarized"
elif [[ -n "${APPLE_ID:-}" ]] && [[ -n "${APPLE_PASSWORD:-}" ]] && [[ -n "${APPLE_TEAM_ID:-}" ]]; then
  NOTARIZE=1
fi

echo "==> WriteCraft $VERSION for macOS ($ARCH), identity: $IDENTITY, notarize: $NOTARIZE"

# ---- build -------------------------------------------------------------------------------------
if [[ "$SKIP_BUILD" = 0 ]]; then
  args=()
  for t in "${TARGETS[@]}"; do args+=(--target "$t"); done
  (cd "$ROOT" && cargo build --release --locked -p wordcraft "${args[@]}")
fi

rm -rf "$WORK"
mkdir -p "$WORK/bin"
inputs=()
for t in "${TARGETS[@]}"; do inputs+=("$CARGO_TARGET_DIR/$t/release/wordcraft"); done
lipo -create -output "$WORK/bin/wordcraft" "${inputs[@]}"
lipo -info "$WORK/bin/wordcraft"

sign() {
  local ts=(--timestamp)
  [[ "$IDENTITY" = "-" ]] && ts=(--timestamp=none)
  local kc=()
  [[ -n "${MACOS_KEYCHAIN:-}" ]] && kc=(--keychain "$MACOS_KEYCHAIN")
  codesign --force --sign "$IDENTITY" ${kc[@]+"${kc[@]}"} "${ts[@]}" "$@"
}

notarize() {
  local file="$1"
  echo "==> notarizing $(basename "$file")"
  xcrun notarytool submit "$file" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" --wait --timeout 1h
}

# ---- WriteCraft.app ----------------------------------------------------------------------------
echo "==> assembling $APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$WORK/bin/wordcraft" "$APP/Contents/MacOS/WriteCraft"
cp "$ROOT/assets/writecraft-icon/writecraft.icns" "$APP/Contents/Resources/WriteCraft.icns"
copy_docs "$APP/Contents/Resources"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@SHORT_VERSION@/$SHORT_VERSION/g" \
  -e "s/@BUILD_SHA@/${WORDCRAFT_BUILD_SHA:-unknown}/g" \
  "$HERE/Info.plist.in" >"$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' >"$APP/Contents/PkgInfo"

# codesign --strict rejects extended attributes; nothing after this may write xattrs (#525).
xattr -cr "$APP"
sign --options runtime --entitlements "$ROOT/packaging/macos/entitlements.plist" "$APP/Contents/MacOS/WriteCraft"
sign --options runtime --entitlements "$ROOT/packaging/macos/entitlements.plist" "$APP"
codesign --verify --strict --deep --verbose=2 "$APP"

if [[ "$NOTARIZE" = 1 ]]; then
  ditto -c -k --keepParent "$APP" "$WORK/WriteCraft-notarize.zip"
  notarize "$WORK/WriteCraft-notarize.zip"
  xcrun stapler staple "$APP"
fi

# ---- DMG ---------------------------------------------------------------------------------------
echo "==> building $DMG"
STAGE="$WORK/dmg"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/WriteCraft.app"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
# hdiutil fails now and then with "Resource busy" on CI runners: retry.
for attempt in 1 2 3 4 5; do
  if hdiutil create -srcfolder "$STAGE" -volname "WriteCraft" -fs HFS+ -format UDZO \
    -imagekey zlib-level=9 -ov "$DMG"; then
    break
  fi
  [[ "$attempt" = 5 ]] && { echo "error: hdiutil create failed 5 times" >&2; exit 1; }
  warn "hdiutil create failed (attempt $attempt of 5); retrying in $((attempt * 10))s"
  sleep $((attempt * 10))
done
sign "$DMG"
codesign --verify --strict --verbose=2 "$DMG"
if [[ "$NOTARIZE" = 1 ]]; then
  notarize "$DMG"
  xcrun stapler staple "$DMG"
fi

echo "==> done"
ls -lh "$DMG"
