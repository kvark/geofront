# Quaternius mechs → Geofront (GLB)

CC0 Animated Mech Pack (Quaternius, March 2021), converted FBX → **GLB** with skins + full animation clips.

Source: https://quaternius.com/packs/animatedmech.html  
Mirror: https://opengameart.org/content/animated-mech-pack  
License: CC0 1.0

## Models (drop the `.glb` files next to this README)

| File | Role | Clips | Notes |
|------|------|-------|--------|
| `Stan.glb` | Player light | 18 | Full arms + legs |
| `Mike.glb` | Player heavy | 18 | Full arms + legs |
| `George.glb` | Enemy heavy | 20 | Arms + `MidLeg` |
| `Leela.glb` | Enemy scout | 18 | **No arm bones** (walker) |

Clips include: Idle, Walk, Run, Punch, Kick, Shoot, Jump, Death, HitRecieve_1/2, Pickup, Dance, Hello, Yes, No, SwordSlash (+ Holding/Tall variants).

## Scale

- Grid cell = **2.0** world units.
- Target height ≈ **2.8** → use **`scale = 0.4`** (already set in `spawn_mech_silhouette`).
- If feet sink slightly, raise spawn Y by ~`0.4`.

## Bone → `LimbKind`

| `LimbKind` | Bones |
|------------|--------|
| `Torso` | `Body`, `Torso`, `Chest`, `Neck`, `Head` |
| `LeftArm` | `Shoulder.L`, `UpperArm.L`, `LowerArm.L`, … |
| `RightArm` | `Shoulder.R`, `UpperArm.R`, `LowerArm.R`, … |
| `LeftLeg` | `UpperLeg.L`, `LowerLeg.L` (`MidLeg.L` on George/Leela), `Foot.L` |
| `RightLeg` | same with `.R` |

Limb damage still drives HUD / mobility / firepower. Visual limb break needs mesh split or bone hide (not in Blade yet).

## Clip → combat

| Moment | Clip |
|--------|------|
| Idle | `Idle` |
| Move | `Walk` / `Run` |
| Attack | `Punch` / `Shoot` / `SwordSlash` |
| Hit | `HitRecieve_1` / `HitRecieve_2` |
| Destroyed | `Death` |

Playback needs Blade glTF skin + animation sampling. Rest pose already renders via the same `Visual.model` path as Kenney buildings.

## Install

From the project zip `geofront-mechs.zip` (or rebuild with Blender):

```bash
cp models/mechs/*.glb assets/models/mechs/
git add assets/models/mechs/*.glb
git commit -m "Add Quaternius mech GLBs (CC0, skinned + anims)"
git push
```
