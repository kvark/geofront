# Geofront

Mecha tactical city defence and management.

Evangelion-inspired hybrid of XCOM-style base management and *Into the Breach*-style focused mech combat, with strong emphasis on pilot psychology, synchronization, loyalty, and a single detailed living/destructible city.

## Status

- **Combat core** — turn-based mission, limb damage, mobility/firepower from limbs, simple enemy AI, city protection HP, win/lose.
- **Interactive HUD** — unit selection, limb targeting, End Turn / Attack / Reset, live log.
- **Dual target** — native + WASM/WebGL2 scaffolding (Blade).
- **Next** — wire `blade_engine::Engine` so the HUD is presented (needs `assets/shaders`), then simple top-down grid visualisation.

See original design notes: https://github.com/kvark/ideas/blob/master/game/eva.md

## Run

```sh
cargo run --release
```

Headless combat smoke test:

```sh
cargo run -- --smoke
```

Optional ray-traced lighting later (once Engine is wired):

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

## Core pillars

- One detailed city (protection funding, destructible, living)
- Few-unit close-up mech combat with limb/zonal damage + pilot disobedience risk
- Pilot–mech sync + interpersonal loyalty systems
- XCOM-like facilities, research, hangars under the city
