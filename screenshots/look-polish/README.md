# Tokyo-3 look polish — before / after

Captured under lavapipe + Xvfb (`DISPLAY=:99`, 1280×800) from the release binary.

```bash
export DISPLAY=:99
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export LIBGL_ALWAYS_SOFTWARE=1
GEOFRONT_VIEW=battle GEOFRONT_SCREENSHOT=screenshots/look-polish/after-battle.png \
  GEOFRONT_QUIT_AFTER=16 ./target/release/geofront
# same for surface. Grab the window with ffmpeg x11grab just before quit.
```

- **Before** shots: unmodified `origin/main` at `b01c68c`.
- **After** shots: this branch, same view, same default camera, turn 1, no autoplay.
- Battle is the default close-up street corner; surface is the default canyon establishing shot.
