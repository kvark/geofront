# Geofront

Mecha tactical city defence and management.

Evangelion-inspired hybrid of XCOM-style base management and *Into the Breach*-style focused mech combat, with strong emphasis on pilot psychology, synchronization, loyalty, and a single detailed living/destructible city.

## Status

- **Combat core** — turn-based mission, limb damage, mobility/firepower from limbs, simple enemy AI, city protection HP, win/lose.
- **Interactive HUD** — unit selection, limb targeting, End Turn / Attack / Reset, live log (presented via Blade + egui).
- **Blade Engine** — Rasterizer path, dark clear colour, top-down combat camera.
- **Dual target** — native + WASM (assets embedded via `include_dir` + Blade VFS).

See original design notes: https://github.com/kvark/ideas/blob/master/game/eva.md

## Setup

Shaders (required for the windowed build):

```bash
./scripts/fetch-shaders.sh
# or: cp -r ../redline/assets/shaders ./assets/shaders
```

## Run (native)

```bash
cargo run --release
```

Headless combat smoke test (no GPU / shaders required):

```bash
cargo run -- --smoke
```

Optional ray-traced lighting (needs RT hardware + RT shaders):

```bash
GEOFRONT_RT=1 cargo run --release
```

## Web / WASM

The web build **is** the game: same Rust crate, compiled to `wasm32-unknown-unknown`. There is no Vite/React app.

```bash
rustup target add wasm32-unknown-unknown
bash scripts/build-web.sh
# serve dist/ (python -m http.server -d dist, or node scripts/serve.mjs dist)
```

GitHub Pages (`.github/workflows/pages.yml`) builds that WASM and deploys on push to `main`.

## Core pillars

- One detailed city (protection funding, destructible, living)
- Few-unit close-up mech combat with limb/zonal damage + pilot disobedience risk
- Pilot–mech sync + interpersonal loyalty systems
- XCOM-like facilities, research, hangars under the city
