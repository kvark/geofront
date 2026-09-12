# Geofront

Mecha tactical city defence and management.

Hybrid of XCOM-style base management and *Into the Breach*-style focused mech combat, with strong emphasis on pilot psychology, synchronization, loyalty, and a single detailed living/destructible city.

![Surface city](screenshots/city-surface.png)

![Underground Geofront](screenshots/city-underground.png)

![Battle](screenshots/combat.png)

## Status

- **Combat** — turn-based skirmish on an 8×8 street grid. Each unit gets move points (orthogonal steps) then one action (attack / wait). Facing, limb targeting, mobility/firepower from limbs, sequential enemy phase. **Pilot stress** spikes when a player mech takes a wrecked limb or near-death hit; they may refuse the next order once (Eva-flavored combat-log lines + HUD flash / ⚠). **Pilot sync** scales attack damage (high sync buff + crit window; low sync weaker) and shows on HUD/log; high sync (≥85%) also reads a brief Angel core / weak-point flash on Splinter and Mass, and core strikes deal proportional bonus damage with a sharper crack FX/log. Alien archetypes: **Splinter** (kite/harass, range 5) and **Mass** (slow pressure, range 2, **AT Field** absorbs the first two hits with a cyan cage / deflect FX); placeholder Quaternius meshes with Angel-ish silhouette/FX (scale/presence, magenta vs crimson attack language, telegraph glow) until real angel-class art.
- **Presentation** — Quaternius skinned GLBs play Idle / Walk / Punch / Hit / Death via `Engine::set_animation`. Attacks use a telegraph wind-up then strike (Splinter snappy, Mass heavy); hit reactions are deferred to impact; death locks and freezes so wrecks stay down. Tokyo-3 dusk mood: hot horizon glow + purple haze, denser/taller skyscraper canyon, dark asphalt, warm sodium + cyan/magenta neon street lights, procedural window glitter + cool rim in the raster path, depth fog, and a lower ¾ anime combat camera with impact punch. Mech strikes keep warm sodium kick; Angel strikes use distinct magenta (Splinter) / crimson (Mass) impact lights + sparks, per-kind telegraph glow, and debug-line silhouettes (tall spines vs bulk cage) so they read as Angels vs mechs without new meshes. High-sync pilots get a brief core star on those silhouettes (pattern sight); landing a strike cracks the core harder. AT Field deflects use cyan cage FX. Directional contact shadows on (Blade skinned-receiver bias).
- **City** — Kenney surface block + Space Kit underground hangar (pieces abut on edges, no stacked floors).
- **HUD** — view switcher, N/W/E/S step, rotate, attack, wait, end turn (Blade + egui). Web also has an HTML view strip so Pages stays playable if the in-canvas panel fails to composite.
- **Dual target** — native + WASM (assets embedded via `include_dir` + Blade VFS; WASM uses Blade's WebGL2 backend). Pinned to Blade `07338d6` (skinned shadow receivers) (#380 texelFetch present so the canvas is not a decoded-sRGB dark frame; #381 shadow FS + wasm32 GLES profile; #378/#379 buffer-class and canvas color-space).

North star: [docs/DESIGN.md](docs/DESIGN.md). Original pitch notes: https://github.com/kvark/ideas/blob/master/game/eva.md

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

Lavapipe playtest (software Vulkan + Xvfb):

```bash
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export LIBGL_ALWAYS_SOFTWARE=1
GEOFRONT_VIEW=battle GEOFRONT_AUTOPLAY=1 GEOFRONT_QUIT_AFTER=90 cargo run --release
```

`GEOFRONT_AUTOPLAY=1` scripts a skirmish to VICTORY/DEFEAT for headless iteration.


Optional ray-traced lighting (needs RT hardware + RT shaders):

```bash
GEOFRONT_RT=1 cargo run --release
```

Capture a view and quit (used by `scripts/capture-screenshots.sh`):

```bash
GEOFRONT_VIEW=battle GEOFRONT_SCREENSHOT=screenshots/combat.png \
  GEOFRONT_QUIT_AFTER=8 cargo run --release
```

`GEOFRONT_VIEW` is `battle`, `surface`, or `underground`. `GEOFRONT_SCREENSHOT` (any value) implies an 8s quit if `GEOFRONT_QUIT_AFTER` is unset. Pair with Xvfb + `import` to write the PNG.

## Web / WASM

The web build **is** the game: same Rust crate, compiled to `wasm32-unknown-unknown`. Blade presents through **WebGL2** (not WebGPU).

```bash
rustup target add wasm32-unknown-unknown
bash scripts/build-web.sh
# serve dist/
```

GitHub Pages (`.github/workflows/pages.yml`) builds that WASM and deploys on push to `main`.

Open `?view=battle`, `?view=surface`, or `?view=underground` to pick the starting camera. The top-left strip does the same at runtime.

## Controls

| Input | Action |
|-------|--------|
| WASD / arrows | Fly camera |
| Drag | Look |
| Q / E | Up / down |
| Shift | Sprint |
| Wheel | Dolly |
| HUD N/W/E/S | Step selected mech one tile |
| ↺ ↻ | Face |
| Attack / Wait / End Turn | Action economy |

## Assets

- Kenney city kits (CC0) under `assets/models/{roads,commercial,industrial,space}`
- Quaternius Animated Mech Pack (CC0) under `assets/models/mechs/` (Stan, Mike, George, Leela GLBs + external albedo PNGs; metalness 0, mild shadow-lift for lavapipe raster)

## Core pillars

- One detailed city (protection funding, destructible, living)
- Few-unit close-up mech combat with limb/zonal damage + pilot disobedience risk
- Pilot–mech sync + interpersonal loyalty systems
- XCOM-like facilities, research, hangars under the city

## Roadmap (from the design notes)

Priority tags from [eva.md](https://github.com/kvark/ideas/blob/master/game/eva.md) (see also [DESIGN.md](docs/DESIGN.md)):

- **(high) GUI** — battle HUD, view switch, log. In-canvas egui plus HTML chrome on wasm.
- **(med) City** — one large detailed block; living + destructible still ahead.
- **(low–med) Battle** — skirmish + Quaternius clips + Splinter/Mass alien AI are in; more alien art / animation still ahead.
- **(low–med) Characters** — pilots carry sync/loyalty/stress; wrecked limbs / near-death spike stress and may skip the next order with Eva-flavored refuse drama lines + HUD flash; high sync buffs damage/crit and flashes Angel cores, low sync weakens strikes; portraits and full dialogs still ahead.
- **(low) Base** — underground hangar is a stage, not a facility sandbox yet.
