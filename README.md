# LaserIO — a Horizon port

**LaserIO** is a Minecraft mod. Can't have DireWire without Wires!. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Direwolf20-MC/LaserIO](https://github.com/Direwolf20-MC/LaserIO) — licence **MIT**.

| | |
|---|---|
| Upstream mod | LaserIO ([source](https://github.com/Direwolf20-MC/LaserIO)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `laserio.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The node.** `laserio:laser_node` (template `minecraft:stone`, hardness 2.0
- **The connector.** `laserio:laser_connector` (its carried model and state
- **The card.** `laserio:card_item` (the carried card texture with the insert
- **The transfer.** After the level's tick, each EXTRACT card whose 20-tick
- **The rate.** Upstream's defaults with no upgrades: **one item every 20
- **The readout.** `/laserio info <x> <y> <z>` prints the cards (side, mode,
- **Containers are the `ITEMS` capability**, not `native_container_slots`/`native_move_items`:
- **One transaction per network per tick.** A card's move is: extract from the source slot, insert

## Not yet ported

- **Energy, fluid and chemical cards** (block-to-block FE works at the pin via
- **Channels as routing**: a card's channel is stored and printed but not yet
- **Filters** (basic/count/mod/NBT/tag) and the count/stock behaviours.
- **Stockers, sensors, redstone cards**, the redstone modes and channels.
- **Overclockers** (card and node): more cards per side and a faster cadence.
- **The node/card menus and screens**, and the advanced connector pairing.
- **The wrench** (linking by hand, connection removal, the colour/particles).
- **Item-flow particles** (upstream's laser beam), the GUI/button/book assets,

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `laserio.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

LaserIO was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Direwolf20-MC/LaserIO

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run laserio` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/laserio/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
