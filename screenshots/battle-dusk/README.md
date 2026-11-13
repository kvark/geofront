# Battle dusk grade + team armor — before / after

Captured under lavapipe + Xvfb (`DISPLAY=:99`, 1280×800) from the release binary.

```bash
export DISPLAY=:99
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export LIBGL_ALWAYS_SOFTWARE=1
GEOFRONT_VIEW=battle GEOFRONT_SCREENSHOT=screenshots/battle-dusk/after-battle.png \
  GEOFRONT_QUIT_AFTER=16 ./target/release/geofront
# same for surface. Grab the window with ffmpeg x11grab (`-draw_mouse 0`) just before quit.
```

- **Before** shots: unmodified `origin/main` at `9622bf5` (Tokyo-3 visual polish).
- **After** shots: this branch, same view, same default camera, turn 1, no autoplay.
- Battle is the default close-up street corner; surface is the default canyon establishing shot (confirming it did not go too dark).
