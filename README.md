# laserio (Horizon port)

A Horizon port of [laserio](Direwolf20-MC/LaserIO) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- The node.** `laserio:laser_node` (template `minecraft:stone`, hardness 2.0
- from upstream's `strength(2.0f)`, no occlusion, 3/16–13/16 outline, the
- carried Blockbench model and its two-frame animated texture). A placed node
- gets a block entity that saves **one card per side**, encoded as
- `Card::saved`.
- The connector.** `laserio:laser_connector` (its carried model and state
- rotations) with a `facing` state; the connector's inventory is the container
- on its `facing` side. It joins the node's network when it touches it.

## Not yet ported

- Implemented: the node, connector and item card blocks/items with carried
- content; per-side cards saved in the block entity; the contiguous-block
- network; EXTRACT → network insertion (node INSERT cards and a connector's
- `facing` container); the 20-tick / 1-item operation with a configurable amount;
- right-click card placement; `/laserio info|card|remove|mode|rate|face`;
- persistence across a relaunch.
- Not implemented (planned slices):
- Energy, fluid and chemical cards** (block-to-block FE works at the pin via

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: Direwolf20-MC/LaserIO. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
