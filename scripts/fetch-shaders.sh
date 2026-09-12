#!/usr/bin/env bash
# Re-sync Blade shaders from blade-render/code at the Cargo.toml rev.
# Note: geofront patches raster.wgsl sky to Tokyo-3 twilight — re-apply that
# gradient after a sync (search for "Tokyo-3 twilight" in raster.wgsl).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REV="07338d682aa403b8dd8e28e82b832d084139dd35"
mkdir -p "$ROOT/assets/shaders"
BASE="https://raw.githubusercontent.com/kvark/blade/${REV}/blade-render/code"
for f in a-trous.wgsl brdf.inc.wgsl camera.inc.wgsl color.inc.wgsl \
         debug-blit.wgsl debug-draw.wgsl debug-param.inc.wgsl debug.inc.wgsl \
         env-importance.inc.wgsl env-light.inc.wgsl env-prepare.wgsl \
         fill-gbuf.wgsl gbuf.inc.wgsl hit.inc.wgsl noop.wgsl \
         path-trace.wgsl post-proc.wgsl quaternion.inc.wgsl random.inc.wgsl \
         raster.wgsl ray-trace.wgsl sampling.inc.wgsl surface.inc.wgsl \
         skin.wgsl skin.inc.wgsl vertex.inc.wgsl; do
  echo "  $f"
  curl -fsSL "$BASE/$f" -o "$ROOT/assets/shaders/$f"
done
echo "Synced $(ls "$ROOT/assets/shaders"/*.wgsl | wc -l) shaders."
