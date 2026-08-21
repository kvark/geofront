# Assets for Geofront

## shaders/

Vendored Blade WGSL shaders from redline (same `blade-engine` revision).

To re-sync from redline at any time:

```bash
chmod +x scripts/fetch-shaders.sh
./scripts/fetch-shaders.sh
```

Native loads from `assets/shaders/`. For WASM, embed the same tree via `include_dir!` + VFS (as redline does).

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

Copy from the curated set in the project’s `geofront-models/` / `geofront-push/assets/models/` if they are missing after a fresh clone.
