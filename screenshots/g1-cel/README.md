# G1 cel shading: toon ramp + ink outline — before / after

Captured under lavapipe + Xvfb (`DISPLAY=:99`, 1280×800) from the release binary,
default cameras, turn 1, no autoplay, `GEOFRONT_QUIT_AFTER=16`, grabbed with
ffmpeg x11grab just before quit (same path as `screenshots/battle-frame/`).

- **Before**: unmodified `origin/main` at `f62857e`.
- **After**: this branch, same views.

Technique: Blade's raster pass draws straight into the swapchain with a private
depth target and no post hook, so a screen-space depth/normal edge pass is not
possible without a Blade change. Outlines are an inverted hull instead: each
outlined visual is spawned twice, the copy with a negative scale; the
`raster.wgsl` overlay reads the mirrored model matrix as the hull flag,
un-mirrors it, extrudes ~3 px along normals, keeps back faces and paints ink.
