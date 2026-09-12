# Geofront — design north star

**Evangelion-inspired mecha tactical city defence** — Tokyo-3 dusk street
battles, underground Geofront base, pilot sync / loyalty / stress. Original
game; not a licensed remake or episode retelling.

Canonical pitch notes: [`eva.md`](https://github.com/kvark/ideas/blob/master/game/eva.md).

## Pillars

- **One detailed city** — surface block you fund and may destroy; underground
  Geofront facilities (hangar, research, shield) under it.
- **Few-unit close-up mech combat** — *Into the Breach* / MechCommander energy:
  limb/zonal damage, facing, telegraph → strike, readable hero mechs on an 8×8
  street grid.
- **Pilots as systems** — sync with the machine, interpersonal loyalty, stress /
  disobedience risk. Characters are the drama; mechs are the tools.
- **XCOM-like base loop** — real time outside, paused in base; protection level
  funds the program.
- **Native + web parity** — lavapipe / Vulkan and WASM WebGL2 share the same
  combat and lighting story (Blade raster; shadows on).

## Look (locked)

**Tokyo-3 twilight street battle**, not noon sandbox or generic sci-fi grey.

| Cue | Intent |
| --- | --- |
| Sky | Hot sodium horizon band → purple haze shelf → deep indigo zenith |
| City | Dense / tall skyscraper canyon framing the 8×8; lit windows at night |
| Street | Dark asphalt so lights + sky pop; warm sodium pools + cool neon accents |
| Mechs | Heroic readability: team tints, cool rim / fresnel, impact flash — not grain silhouettes |
| Angels | Distinct magenta/crimson strike language + debug-line presence; Mass AT Field cyan cage |
| Camera | Low ¾ anime combat framing; surface overview as canyon establishing shot |
| Atmosphere | Soft depth haze toward dusk; contact shadows without blacking skinned mechs |

Reference fantasy: Evangelion / Tokyo-3 city battle stills. Art budget is Kenney
city kits + Quaternius mechs (CC0) — aim for *anime-adjacent screenshots*, not
film stills.

## Mechanics (locked direction)

| Layer | Direction |
| --- | --- |
| Combat | Turn-based on grid: move points then one action; facing; limb targeting; sequential enemy phase |
| Aliens | Splinter kite + Mass pressure; Mass AT Field absorbs first hits; Angel FX until real art; high-sync core telegraph + core-strike bonus |
| City | Destructible / living city ahead; collateral already affects protection % |
| Base | Underground hangar is a stage today; facility sandbox is the roadmap |
| Pilots | Sync / loyalty / stress already on HUD; high sync buffs damage, reads Angel cores, and core-strikes for bonus damage; portraits, dialogs, disobedience next |

## Non-goals

- Remaking Evangelion episodes, units, or UI frame-for-frame.
- Disabling directional shadows to “fix” lavapipe (keep Blade skinned-receiver path).
- Massive proprietary art packs; prefer small CC0 kits + procedural / tint / light tricks.
- Wasm-only presentation paths that diverge from native lavapipe playtests.

## Presentation checklist (playtest)

When judging a build under lavapipe:

1. Does the mid-battle shot read as **dusk canyon** (warm key, cool fill, sky glow)?
2. Do **sodium / neon** pools and window glitter sell night city?
3. Are player mechs **readable heroes**, not grain silhouettes?
4. Do Angels read vs mechs (silhouette/FX, AT Field cyan cage, high-sync core flash / crack) without new meshes?
5. Does `GEOFRONT_AUTOPLAY=1` still reach **VICTORY**?
