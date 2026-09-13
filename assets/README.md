# Assets for Geofront

## shaders/

**Do not vendor the full Blade WGSL tree.** Stock shaders load from
`blade_render::shader_dir()` (packaged in the `blade-render` crate).

This directory holds **game-only overlays**. Today that is a single file:

| File | Purpose |
| --- | --- |
| `raster.wgsl` | Tokyo-3 twilight sky + anime rim / window glitter / dusk fog |

### How overlays layer

1. **Native** — `blade_render::shader_dir()` is the stock base. If any
   `assets/shaders/*.wgsl` exist, they are copied over a materialization of
   that base into `asset-cache/shaders/`, and Engine `shader_path` points there.
2. **WASM** — `build.rs` merges the same stock + overlays into
   `$OUT_DIR/merged-shaders`, which `include_dir!` embeds and mounts at
   `assets/shaders` in the VFS (models still come from `include_dir!` of
   `assets/`).

When bumping the Blade git rev, re-diff `raster.wgsl` against
`blade-render/code/raster.wgsl` and re-apply the Tokyo-3 hunks if stock drifted.

## models/

Kenney CC0 city + Space Kit pieces used by the battle / city views.

Layout (each folder needs a sibling `Textures/colormap.png` because the GLBs reference `Textures/colormap.png`):

```
models/
  roads/          # road-straight, road-crossing, road-crossroad, …
  commercial/     # skyscrapers + mid-rise
  industrial/     # chimneys, tanks, low buildings
  space/          # underground corridors, rooms, gates
```

Copy from the curated set in the project's `geofront-models/` / `geofront-push/assets/models/` if they are missing after a fresh clone.
