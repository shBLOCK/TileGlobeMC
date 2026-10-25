# TileGlobeMC

-----

[Demo Video](https://youtu.be/YhDgAMrMPlI)

TODO: Description

Project Structure
-----
- tileglobe: game logic
- datagen/mod: a fabric modding environment for extracting/generating data from vanilla (e.g. blockstate info, networking packet IDs, etc.)
- tileglobe_pymeta: Python environment and helper package for [pymeta](https://github.com/shBLOCK/pymeta) usage in the project
- tileglobe_server: Minecraft server protocol implementation
- tileglobe_runtime: runtime platform implementation
  - pc
  - rp2350: runnable on a [Pimoroni Pico Plus 2 W](https://shop.pimoroni.com/en-us/products/pimoroni-pico-plus-2-w)
