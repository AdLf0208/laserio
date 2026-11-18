# LaserIO 1.9.11 → Horizon: findings from the jar and upstream

## Provenance

- Pack jar: `ports/_shared/licence-cache/laserio-1.9.11.jar`, **1,305,285 B**,
  SHA-256 `03e8537d75bc2f4ced2fc214d3409753e684d1056ee63b26db7a2b9e199ef4df`,
  634 zip entries. Kept as the licence and content-carry evidence.
- `META-INF/neoforge.mods.toml`: `modId="laserio"`, `version="1.9.11"`,
  `license="MIT (https://github.com/Direwolf20-MC/LaserIO/blob/main/LICENSE.txt)"`,
  display name **LaserIO**, authors Direwolf20/ErrorMikey, MC `[1.21,1.22)`,
  optional Mekanism integration. Licence read from the jar: **MIT**, rank 0.
- Upstream repository <https://github.com/Direwolf20-MC/LaserIO>, shallow clone
  `ports/_shared/upstream/laserio` at commit
  `4d278a6351a434d343fbf70d942baeeaf2607536` (2024-09-16), whose
  `gradle.properties` says `mod_version=1.9.11` — the exact jar version.
  Deleted after these findings were recorded. No Java was copied or translated
  and the jar was never decompiled.

## Jar inventory (what the mod is)

Registry entries visible in the jar (`assets/laserio/models/item/`,
`blockstates/`, `lang/en_us.json`):

- **Blocks (3):** `laser_node`, `laser_connector`, `laser_connector_advanced`.
- **Items:** `card_item`, `card_fluid`, `card_energy`, `card_chemical`,
  `card_redstone`, `card_cloner`, `card_holder`; `filter_basic`,
  `filter_count`, `filter_mod`, `filter_nbt`, `filter_tag`; `laser_wrench`;
  `logic_chip`, `logic_chip_raw`; `overclocker_card`, `overclocker_node`.
- **Recipes:** one per item/block plus `_nbtclear` copies; shaped crafting and
  one smelting recipe (`logic_chip` from `logic_chip_raw`, 100 ticks).
- **Loot tables:** the three blocks, self-drop.
- **Assets:** block models are large Blockbench models (`laser_node` has a
  `tintindex: 1` core), animated `laser_node.png` (2 frames, 25 ticks each);
  GUI/button textures and a Patchouli guide book.
- No `LICENSE` file inside the jar; the licence text is the upstream repo's
  `LICENSE.txt` (MIT, Copyright (c) 2022 Direwolf20-MC).

## Upstream transfer semantics (the behaviour this slice ports)

Read in `LaserNodeBE` / `BaseCard` / `CardItem` / `ExtractorCardCache` /
`InserterCardCache` / `LaserWrench` / `LaserNode` / `LaserConnector`.

- **Model.** A `laser_node` is a block entity with **six sides**; each side has
  **nine card slots** (`LaserNodeContainer.CARDSLOTS = 9`). A card placed on a
  side sees the inventory **on that side**. `laser_connector` is a relay with a
  `facing` state and no cards of its own; `laser_connector_advanced` pairs two
  connectors across a distance.
- **Network.** Connections are explicit, made with the **laser wrench**: click
  one node (sneak) to store its position, click another to link them; links are
  refused beyond `maxDistance = 8` blocks and across dimensions. The node
  discovers its network by walking the stored links
  (`BaseLaserBE.discoverAllNodes`).
- **Cards.** `BaseCard.TransferMode` is `INSERT` (0, the default),
  `EXTRACT` (1), `STOCK` (2), `SENSOR` (3). Channel is a byte, 0–15, default 0.
  Cards are one stack each. Right-clicking a node with a card inserts it into
  the slot group of the **clicked face** (`LaserNode.useWithoutItem`).
- **Rate (upstream's defaults, no upgrades):** `CardItem.getItemExtractAmt`
  answers **1** when the component is absent; `BaseCard.getExtractSpeed`
  answers **20** ticks. `ExtractorCardCache.remainingSleep` is reloaded to
  `tickSpeed` after each operation, so one extractor card moves **one item
  every 20 ticks**. A node overclocker raises `overClocker`, letting that many
  cards on a side operate in the same tick (not this slice).
- **Extraction to insertion.** `sendItems` scans the extractor side's inventory
  slots, takes the first stack the card accepts, copies **`extractAmt`** of it,
  and `extractItem` (1) fake-extracts, (2) finds every **inserter card with the
  same channel** in the network that accepts the stack, (3) test-inserts into
  each in order until the batch fits, and only then (4) really extracts and
  inserts. **If nothing fits, nothing is extracted** — the source keeps its
  items. Nearest-first is *not* the default here: inserter order is network
  discovery order, and round-robin/priority are card settings.
- **Redstone.** Cards have a redstone mode/channel; an `enabled` flag gates
  every operation. Default is "ignore" (always enabled). Sensors and stockers
  build on the same tick, out of this slice.
- **Filters.** A card may hold a filter item (basic/count/mod/NBT/tag); a card
  with no filter accepts any stack. Filter UI is a gap.
- **Item flow is a rendered particle** from node to node (`ParticleData`);
  there is no item entity in the world.
- **Screens.** The node opens a per-side menu with nine card slots and settings
  buttons; cards open their own menu; a wrench removes the connection.

## What slice 1 implements, and why

**Slice:** the `laser_node` block with **one item card in EXTRACT mode** on one
side, a `laser_connector` touching the node, and the container the connector
**faces**, with upstream's default **1 item per 20-tick operation** (the rate
is pinned by a runtime test and by the e2e; `/laserio rate` raises it to 64 for
machines that need throughput).

Reason: this is the smallest observable end-to-end LaserIO behaviour — a card
extracting from an inventory and a network inserting into another — and every
later slice (insert cards, channels, filters, stockers, sensors, fluid/energy
cards, overclockers, the menu) is additive on the same node/connector/card
shape. Doing the whole card/filter system first would be a machines-tier
campaign of its own.

## Deliberate deviations (recorded, not hidden)

| Upstream | Here | Why |
|---|---|---|
| Network links made with the wrench, up to 8 blocks | a node/connector network is the **contiguous run** of touching laser blocks | the wrench is another item + a held-position component + a screenless UX; touching blocks are enough for one node and one connector, and the pin has no item-data write path for the wrench state |
| A connector only relays; insertion needs an **inserter card** on a node | the connector inserts through its `facing` side | the brief's slice is *one card + one connector*; a second card adds nothing this slice proves. `INSERT` cards are implemented too, so the faithful two-card path works |
| Nine cards per side, one menu per side | one card per side, set by `/laserio card`/`mode`/`rate` | no screen work in this slice; the per-side model is kept |
| Card settings live in item components (mode/channel/amount) | settings live in the node's block entity per side | the pin has no item-component write helper for arbitrary data; the observable transfer is the same |
| Per-card `remainingSleep` counter | cadence derived from `game_time + position + side` | block-entity writes made inside the tick are reverted at the pin (`ARCHITECTURE.md` §3) |
| 8 filter/card/redstone pixel features | absent | later slices; the card accepts any stack |
