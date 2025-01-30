TODO

[ ] generate puzzle in thread
[ ] new icon set
  - configurable icon set!
[x] stop timer when puzzle completed
[x] center puzzle
[ ] random strategy for different varieties of puzzle generation (use more left of clues, more 1 but not both, etc.)
  - Maybe add heuristic to reinforce certain decisions
[ ] Selectable theme
[ ] auto-rescale assets for optimal display on HD displays
  - react to fractional scaling, etc.
[ ] Show seed in UI, copy button, allow new puzzle from seed option

# Good seeds

(so far)

good seed 14913497800492826408

# Notes on windows build

- need packages `gcc-mingw-w64`, `mingw-w64`, `nsis` (nullsoft installer), and some dev deps probably.
- Download gtk for windows, extract to `./bundle/gtk`
- Download VulkanRT-{ver}-Components.zip; extract to `./bundle/vulkan`
  - Maybe need to install SDK for windows and copy this? IDK. I did it but the file seems to be in VulkanRT components. `~/.wine-affinity/drive_c/windows/system32/vulkan-1.dll` to `./bundle/gtk/lib/vulkan-1.dll`
- run `./config-windows.sh` to finish setting up the files and editing paths in the package conf files.

- run `make windows`.

# Linux build
