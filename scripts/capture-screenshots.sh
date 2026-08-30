#!/usr/bin/env bash
# Capture Battle / Surface / Underground under Xvfb + Lavapipe.
# Requires: xvfb, ImageMagick `import`, a release binary.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${GEOFRONT_BIN:-$ROOT/target/release/geofront}"
OUT="$ROOT/screenshots"
ICD="${VK_ICD_FILENAMES:-/usr/share/vulkan/icd.d/lvp_icd.json}"
DISPLAY_NUM="${DISPLAY_NUM:-99}"
WAIT="${GEOFRONT_QUIT_AFTER:-10}"

if [[ ! -x "$BIN" ]]; then
  echo "missing binary: $BIN" >&2
  echo "build with: cargo build --release" >&2
  exit 1
fi
mkdir -p "$OUT"

if [[ -z "${DISPLAY:-}" || "${DISPLAY}" != ":${DISPLAY_NUM}" ]]; then
  if ! pgrep -f "Xvfb :${DISPLAY_NUM}" >/dev/null; then
    Xvfb ":${DISPLAY_NUM}" -screen 0 1280x800x24 -ac +extension GLX >/tmp/xvfb-gf.log 2>&1 &
    sleep 0.4
  fi
  export DISPLAY=":${DISPLAY_NUM}"
fi
export VK_ICD_FILENAMES="$ICD"
export LIBGL_ALWAYS_SOFTWARE=1

capture() {
  local view="$1" dest="$2"
  echo "capturing $view -> $dest"
  GEOFRONT_VIEW="$view" GEOFRONT_SCREENSHOT="$dest" GEOFRONT_QUIT_AFTER="$WAIT" \
    RUST_LOG=info "$BIN" >/tmp/gf-"$view".log 2>&1 &
  local pid=$!
  local frames=0
  while kill -0 "$pid" 2>/dev/null && [[ $frames -lt $WAIT ]]; do
    sleep 1
    frames=$((frames + 1))
  done
  # Grab the last painted frame before the process exits.
  import -display "$DISPLAY" -window root "$dest" || true
  wait "$pid" || true
  echo "  wrote $dest ($(wc -c < "$dest") bytes)"
}

capture battle "$OUT/combat.png"
capture surface "$OUT/city-surface.png"
capture underground "$OUT/city-underground.png"
echo "done."
