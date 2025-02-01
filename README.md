# GnomeClu

GnomeClu is a graphical implementation of the [Zebra Puzzle](https://en.wikipedia.org/wiki/Zebra_Puzzle), built with GTK. It runs on Linux and Windows.

# How to Play

![GnomeClu Hard Puzzle](docs/6x6-screeen.png)

GnomeClu is a logical deduction style puzzle, similar to Suduko. Your goal is to find out the correct location of tiles in the puzzle grid, using deductions derived from the provided clues.

The game board is shown above as the 6x6 grid, with each cell containing six candidate tiles.

# Backgound

GnomeClu takes pretty heavy inspiration by [Everett Kaser's Sherlock](https://www.kaser.com/home.html). The creator of this game has played 1,000s of hours of Everett's game over the decades. My grandparents played Kaser's game, my parents still play it, and so do I.

Current differences:

- Puzzle generator
  - Puzzle generator more reliably generates difficult puzzles, with "veteran" difficulty puzzles taking experienced solvers about 1 hour to solve.
  - Puzzles often require advanced deductions such as converging solutions, hidden pair deductions. Puzzle solver is sometimes expected to discount potential solutions which create invalid puzzle states.
  - Puzzle variety system which guides the generator using heuristics to change the overall deduction pattern themes. 
- More subtle clue system which nudges the user where to look, with incremental help.
- A stupid amount of possible puzzles... 18,446,744,073,709,551,615 of them.


Planned differences:

- "X-ray" feature
- Experimental new clue types.
- Competition mode (two users playing the same puzzle, racing on time).

If you enjoy this game, please buy Everett's mobile version for your phone / iPad. He's a smart guy and it's worth your money. Also, check out his other puzzle games.

There are no plans to make a mobile version of GnomeClu.

# Building

GnomeClu is built with Rust and GTK4+. You should install a modern version of Rust using `rustup`.

## Notes on windows build

- need packages `gcc-mingw-w64`, `mingw-w64`, `nsis` (nullsoft installer), and some dev deps probably.
- Download gtk for windows, extract to `./bundle/gtk`
- Download VulkanRT-{ver}-Components.zip; extract to `./bundle/vulkan`
  - Maybe need to install SDK for windows and copy this? IDK. I did it but the file seems to be in VulkanRT components. `~/.wine-affinity/drive_c/windows/system32/vulkan-1.dll` to `./bundle/gtk/lib/vulkan-1.dll`
- run `./config-windows.sh` to finish setting up the files and editing paths in the package conf files.

- run `make windows`.

## Linux build

`make linux` oughta do it. You'll probably get yelled at for some missing dev dependencies. Install them, repeat until success. You can do it. I believe in you.

## Mac OS build

While GTK has access to Apple computers, the author of this game does not. If someone wants to help here, have a look at https://docs.gtk.org/gtk4/osx.html; You'll probably want to make a fat binary and bundle a build of GTK as is done for the Windows port.
