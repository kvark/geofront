# Assets for Geofront

Blade Engine expects:

- `shaders/` — the raster (and optionally RT) shaders used by blade-render / blade-engine
- `models/` — glTF/GLB mechs, city tiles, props (later)
- optional environment maps, particles, etc.

## Quick start

The easiest way to get a working presentation path is to reuse the shader set from redline (or from blade’s own examples):

```sh
# from the geofront root
cp -r ../redline/assets/shaders ./assets/shaders
# or point the Engine config at redline’s shaders while developing
```

Then in `src/main.rs` (or a future `render::Scene`) initialise:

```rust
let engine = blade_engine::Engine::new(
    blade_engine::Presentation::Window(&window),
    &blade_engine::config::Engine {
        shader_path: "assets/shaders".into(),
        data_path: "assets".into(),
        cache_path: "asset-cache".into(),
        time_step: 0.01,
        render_backend: blade_engine::config::RenderBackend::Rasterizer,
        gui_enabled: true,
    },
);
```

After that, the existing egui tessellation + `engine.render(camera, primitives, textures_delta, size, scale)` path from redline will paint the combat HUD and any 3D placeholders.

Models can stay empty for a long time — a clear colour + egui is enough for the combat vertical slice.
