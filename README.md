# Geofront

Mecha tactical city defence and management.

Evangelion-inspired hybrid of XCOM-style base management and *Into the Breach*-style focused mech combat, with strong emphasis on pilot psychology, synchronization, loyalty, and a single detailed living/destructible city.

## Status

- **Combat core** — turn-based mission, limb damage, mobility/firepower from limbs, simple enemy AI, city protection HP, win/lose.
- **Interactive HUD** — unit selection, limb targeting, End Turn / Attack / Reset, live log (presented via Blade + egui).
- **Blade Engine** — Rasterizer path, dark clear colour, top-down combat camera. Ready for grid/mech placeholders.
- **Dual target** — native + WASM/WebGL2 scaffolding.

See original design notes: https://github.com/kvark/ideas/blob/master/game/eva.md

## Setup

Blade needs the raster shaders. Easiest source is redline (or blade’s own examples):

```sh
mkdir -p assets
cp -r ../redline/assets/shaders ./assets/shaders
# optional later: models/, etc.
```

## Run

```sh
cargo run --release
```

Headless combat smoke test (no GPU / shaders required):

```sh
cargo run -- --smoke
```

Optional ray-traced lighting (needs RT hardware + RT shaders):

```sh
GEOFRONT_RT=1 cargo run --release
```

## Web / WASM

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/pkg \
    target/wasm32-unknown-unknown/release/geofront.wasm
# then serve dist/ (see web/index.html)
```

Assets are expected under `assets/`; for WASM they can later be embedded via `include_dir` the same way redline does.

## Core pillars

- One detailed city (protection funding, destructible, living)
- Few-unit close-up mech combat with limb/zonal damage + pilot disobedience risk
- Pilot–mech sync + interpersonal loyalty systems
- XCOM-like facilities, research, hangars under the city
