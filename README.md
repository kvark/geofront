# Geofront

Mecha tactical city defence and management.

Evangelion-inspired hybrid of XCOM-style base management and *Into the Breach*-style focused mech combat, with strong emphasis on pilot psychology, synchronization, loyalty, and a single detailed living/destructible city.

## Status

Early scaffolding. Dual native + WASM/WebGL2 targets via Blade engine.

## Run (native)

```sh
cargo run --release
```

Optional ray-traced lighting (needs RT hardware):

```sh
GEOFRONT_RT=1 cargo run --release
```

## Web / WASM

Requires a WebGL2 browser. Assets will be embedded at compile time.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir dist/pkg \
    target/wasm32-unknown-unknown/release/geofront.wasm
# then serve dist/
```

## Design notes

See original idea: https://github.com/kvark/ideas/blob/master/game/eva.md

Core pillars:
- One detailed city (protection funding, destructible, living)
- Few-unit close-up mech combat with limb/zonal damage + pilot disobedience risk
- Pilot–mech sync + interpersonal loyalty systems
- XCOM-like facilities, research, hangars under the city
