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

Place Kenney CC0 (or custom) `.glb` files here.
Recommended: City Kit (Industrial / Modular) for the city, Space Kit robots as temporary mechs.
