# Assets for Geofront

Blade Engine expects:

- `shaders/` — **required** for the windowed build (raster shaders from blade-render / blade-engine)
- `models/` — glTF/GLB mechs, city tiles, props (later)
- optional environment maps, particles, etc.

## Quick start

```sh
# from the geofront root (adjust path to your redline checkout)
cp -r ../redline/assets/shaders ./assets/shaders
```

After that:

```sh
cargo run --release
```

You should see the dark clear + interactive combat HUD.

The `--smoke` path does not need shaders or a GPU.

## Later

- Placeholder cubes or simple quads for the 8×8 grid and mechs can be added via `engine.add_object` once any GLB (even a unit cube) is present.
- City tiles and animated mechs come after the combat feel is locked.
