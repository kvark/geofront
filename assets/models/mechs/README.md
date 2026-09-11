# Quaternius Animated Mech Pack (CC0)

Source: [Quaternius Animated Mech Pack](https://quaternius.com/) (March 2021).

| File | Role | Animations | Notes |
|------|------|------------|-------|
| `Stan.glb` | Player light | 18 | Full arms + legs |
| `Mike.glb` | Player heavy | 18 | |
| `George.glb` | Enemy Mass | 19 (incl. Tall) | Walk index 16 |
| `Leela.glb` | Enemy Splinter | 18 | |

## Textures

Albedo PNGs live next to the GLBs (`Stan_Texture.png`, …) so Blade's path-based
cook matches Kenney buildings. Sources are the pack `Textures/` maps, resized to
512² with a mild shadow lift for Blade's raster + Reinhard path under lavapipe.

Materials: `metallicFactor=0`, `roughnessFactor≈0.85`, `baseColorFactor≈1.45`, small `emissiveFactor≈0.14` fill so dark armor never drops to pure black under lavapipe.

## Spawn notes

- Pack authored ~7 m tall; geofront scales player mechs ≈0.4 so feet sit on a 2-unit cell.
- If feet sink slightly, raise spawn Y by ~`0.4`.
