# LaserIO → Horizon — slice 1: one laser node moving items

**Format: the SDK package (2026-11-18), version 0.2.0.** This port was migrated from the retired
`.hnmod` container to the flat SDK package — one `.hmod` carrying both the code (`src/`) and the
content (`assets/`, `data/`). Former PR **#872** (the `.hnmod`) was closed by the 2026-11-18 format
purge; 0.2.0 supersedes it. [`## Migration to the SDK format`](#migration-to-the-sdk-format) has the
file map, the old-injection → new-surface table, the traps and the measured ladder.
`src/runtime.rs` is **byte-identical** to the pre-migration `mixins/laserio/runtime.rs` (sha256
`45627ed6074c5d0d821f3fce5fb884370b823eff980026274cc8c30e7805dce5`), including the copy inside the
built package.

Port of **LaserIO 1.9.11** (`Direwolf20-MC/LaserIO`, **MIT**), Minecraft 1.21.1
NeoForge, from the exact ATM10 jar `laserio-1.9.11.jar`, **1,305,285 B**, SHA-256
`03e8537d75bc2f4ced2fc214d3409753e684d1056ee63b26db7a2b9e199ef4df`.
Provenance, the licence read from the jar and the carried files:
[`../_shared/carry-work/laserio/SOURCE.md`](../_shared/carry-work/laserio/SOURCE.md).

**Class + tier:** `content` / **`machines`** (`ports/ledger.csv`; provenance
`ports/_shared/source-map/atm10-sources.csv` row `laserio`, core, P0). Approach
per Horizon's `docs/modpacks.md`: a machines mod is a block entity plus a
capability-style transfer on its own tick, so this is a native slice with a
content half (two blocks, the card item, models, textures, lang, recipes, loot)
rather than a data-only carry.

**Licence:** **MIT → rank 0** (permissive). Local-only until the maintainer
approves publishing (ARCHITECTURE.md §8); a published port travels under MIT
with this licence and its complete source.
**Why now:** the licence-first queue puts a rank-0 core mechanics mod ahead of
copyleft and unknown-licence work; LaserIO is core (no required parent) and its
item routing needs no API the pin does not already have.

**Upstream analysis:** repository <https://github.com/Direwolf20-MC/LaserIO>,
commit `4d278a6351a434d343fbf70d942baeeaf2607536` (the clone whose
`gradle.properties` says `mod_version=1.9.11`; shallow clone in
`ports/_shared/upstream/laserio`, deleted after the findings were written). The
behaviour extracted — node/card/connector model, the channel system, the
20-tick / one-item defaults, "nothing is extracted unless an inserter accepts
it", the wrench network, redstone and filters — is in [`FINDINGS.md`](FINDINGS.md).
No Java was copied or translated and the jar was not decompiled.

## What slice 1 does

- **The node.** `laserio:laser_node` (template `minecraft:stone`, hardness 2.0
  from upstream's `strength(2.0f)`, no occlusion, 3/16–13/16 outline, the
  carried Blockbench model and its two-frame animated texture). A placed node
  gets a block entity that saves **one card per side**, encoded as
  `Card::saved`.
- **The connector.** `laserio:laser_connector` (its carried model and state
  rotations) with a `facing` state; the connector's inventory is the container
  on its `facing` side. It joins the node's network when it touches it.
- **The card.** `laserio:card_item` (the carried card texture with the insert
  and channel layers). Right-clicking a node with a card places it on the
  clicked side, as upstream's `LaserNode.useWithoutItem` does (consumed outside
  creative). A card is `insert` (upstream's default) or `extract`.
- **The transfer.** After the level's tick, each EXTRACT card whose 20-tick
  operation falls on this tick (`game_time + x + y + z + side ≡ 0 mod 20`, the
  pin's substitute for a saved counter) takes the first item type the source
  side offers and moves up to its amount into the network's inventories — a
  node's INSERT card sides and a connector's `facing` side. **No destination
  means no extraction**, and `native_move_items` puts a refused item back, so
  nothing is lost to a full target.
- **The rate.** Upstream's defaults with no upgrades: **one item every 20
  ticks** (`CardItem.getItemExtractAmt` = 1, `BaseCard.getExtractSpeed` = 20).
  `/laserio rate <pos> <side> <n>` raises the amount to 64; a runtime test and
  the e2e both pin the numbers.
- **The readout.** `/laserio info <x> <y> <z>` prints the cards (side, mode,
  channel), the network members, the targets and each card's amount/cadence;
  `/laserio card|remove|mode|rate|face` are the harness settings path
  (upstream's node screen is a gap).

### Deliberate deviations

| Upstream | Here | Why |
|---|---|---|
| Wrench links, up to 8 blocks, across dimensions | the network is the contiguous run of touching laser blocks | the wrench is a second item plus a held-position component and a screenless UX; touching blocks are enough for one node and one connector |
| A connector only relays; insertion needs an INSERT card on a node | the connector inserts through its `facing` side | the brief's slice is one card + one connector; the faithful two-card path also works (INSERT cards are implemented) |
| Nine card slots per side, one menu per side | one card per side, set by `/laserio` | no screen work in this slice; the per-side model is kept |
| Card settings in item components | settings in the node's block entity per side | the pin has no arbitrary item-component write helper; the observable transfer is the same |
| Per-card `remainingSleep` | cadence derived from position + side | a block-entity write inside the tick is reverted at the pin (ARCHITECTURE.md §3) |
| Channels, filters, priorities, round robin, redstone, stockers, sensors | absent; a card accepts any stack and its channel is stored/printed only | later slices; the data model keeps the channel so those slices are additive |

## Slice boundary and gaps

Implemented: the node, connector and item card blocks/items with carried
content; per-side cards saved in the block entity; the contiguous-block
network; EXTRACT → network insertion (node INSERT cards and a connector's
`facing` container); the 20-tick / 1-item operation with a configurable amount;
right-click card placement; `/laserio info|card|remove|mode|rate|face`;
persistence across a relaunch.

Not implemented (planned slices):

1. **Energy, fluid and chemical cards** (block-to-block FE works at the pin via
   `native_move_energy`, API 142, so an energy card is a later slice).
2. **Channels as routing**: a card's channel is stored and printed but not yet
   used to match extract to insert; today every INSERT endpoint in the network
   is a destination.
3. **Filters** (basic/count/mod/NBT/tag) and the count/stock behaviours.
4. **Stockers, sensors, redstone cards**, the redstone modes and channels.
5. **Overclockers** (card and node): more cards per side and a faster cadence.
6. **The node/card menus and screens**, and the advanced connector pairing.
7. **The wrench** (linking by hand, connection removal, the colour/particles).
8. **Item-flow particles** (upstream's laser beam), the GUI/button/book assets,
   and the `laserio:mode` item-model overrides.
9. **Recipes that use the `_nbtclear` variants** and the Patchouli guide book.

Known limits: the connector's outline is a single box, not the carried model's
shape; the connector's `facing` is set by `/laserio face` (upstream takes it from
the placement click, which the pin's hooks do not expose as a block state); the
node's `tintindex` core renders untinted (the pin has no block-colour provider).

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port (slice 1b, packaging); slice 1 recorded superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Migration to the SDK format

*Recipe followed from `ports/imfast/notes.md` → `## Migration to the SDK format` (the first
migration) and `ports/integratedtunnels/notes.md` (the closest analogue: a block entity, a level
tick, the `ITEMS` capability, a transaction and commands). 2026-11-18. Version **0.1.0 (bundle) /
1.9.11 (content) → 0.2.0**; **PR #872** (the `.hnmod`) is superseded.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired the `.hnmod` container, and the
registry closed every `.hnmod` submission on 2026-11-18 with one reason: *"republish this as a
native `.hmod` with the SDK from the next Nightly"*. The new unit is **one flat `.hmod`** — content,
native code, or both — so this port no longer needs the AE2 embedded-content trick, its
`content_root` injection or its `.horizon-cache` write: `assets/` and `data/` sit beside `src/` in
the same package.

**Pins.** SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (surface + `api-source` cached
at `ports/_shared/sdk/0b14f8a/`), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (was
`5a64af93…`), launcher 0.2.78, `package_format` 6, `content_api` 0.2. The old campaign pin
`1d63e0a` does not apply to a package build.

**The tool is run directly in the port.** `ports/tools/build-slot.sh` speaks the retired staging
tool (`tools/horizon.py`, `mixins/`), which this layout no longer has, so every SDK command ran in
`ports/laserio` with `CARGO_INCREMENTAL=0` and no `--target-dir`. A slot lease still wraps each one
(`build-slot.sh run --job sdk-lo --port ports/laserio --step … -- bash -c 'cd ports/laserio && …'`)
so the box-wide concurrency accounting keeps working, but the *command inside* is
`cargo horizon-mod build|check|fits` and `cargo test|clippy|fmt`, never `horizon.py`. That is a
deliberate exception to the staging rule, forced by the format.

### 1. Files created

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity: `id = "laserio"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a` / `e59da90` |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`: `register`, `right_click_block`, `level_tick_post`, `register_commands`, `run_mod_command` |
| `src/laser.rs` | the game glue (the old `support.rs`): the node's block entity **Part**, the tick queue, the once-per-network pass, the cards' NBT, the card placement and the six command handlers |
| `src/runtime.rs` | copied byte-for-byte from `mixins/laserio/runtime.rs` (`cp`, then `cmp`) |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` |
| `assets/laserio/`, `data/laserio/` | the carried content, moved from `content/` (same 24 assets + 13 data files, no separate content build) |
| `README.md`, `LICENSE`, `LICENSE-NOTICE.md` | inside the package (the manifest packs root documents) |
| `tools/e2e.sh` | kept, rewritten for a launcher-built game (`GAME=`, `CONTROL_GAME=`) |

### 2. Files deleted (and where the evidence went)

Deleted **before** the first `cargo horizon-mod build` (trap 1: a leftover old path refuses the
package with `unexpected package path`). Everything is in
`ports/_shared/evidence/laserio/retired-hnmod-0.1.0/`:

| Deleted | Why | Kept as |
|---|---|---|
| `mixins/laserio/` (plan + 6 helpers + `support.rs` + `runtime.rs` + `data/laserio-content.hmod`) | the container is retired; `runtime.rs` moved to `src/` | `retired-hnmod-0.1.0/mixins/` |
| `laserio.hnmod` (31,547 B, the repair build) | unloadable on launcher ≥ 0.2.70 | `retired-hnmod-0.1.0/laserio.hnmod` |
| `content/horizon-mod.toml`, `content/laserio.hmod` (19,421 B), `content/target/` | one manifest and one package now; its `assets/` + `data/` moved to the package root | `retired-hnmod-0.1.0/content-old/` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in the manifest is the pin; the pinned commit decides the toolchain | `retired-hnmod-0.1.0/` |
| `tools/horizon.py`, `package.py`, `package.sh`, `hmod-load.sh`, `add-notices.py`, `carry_content.py`, `tools/audit/` | staging, `.hnmod` packaging, the zip-notice append and the plan audit are all gone; `cargo horizon-mod build` packs `LICENSE` itself | `retired-hnmod-0.1.0/tools/` |
| `Cargo.lock` | not needed without registry deps, and it changes the artifact (trap 5) | not kept |

**Kept unchanged:** `FINDINGS.md`, `LICENSE`, `LICENSE-NOTICE.md`, `VALIDATION.md`, and the
*scenario* of `tools/e2e.sh` (A–F: its assertions are format-independent).

### 3. Manifest field mapping (this port)

| Old (`horizon-mixin.toml` + `content/horizon-mod.toml`) | New (one `horizon-mod.toml`) |
|---|---|
| `[mod] id = "laserio"` | `id` (also the resource namespace) |
| `[mod] version = "0.1.0"` (bundle) and content `1.9.11` | `version = "0.2.0"` — one version for the one package |
| `[mod] priority = 100` | gone; ordering is `#[listener(priority = …)]` |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8ac…"` |
| mixins `5a64af938ec8…` | `[native] mixins = "e59da9094f0c…"` |
| `[[module]] runtime` + `[[module]] state` (`support.rs`) | `src/runtime.rs` and `src/laser.rs` — ordinary Rust modules, `pub mod runtime;` / `mod laser;` |
| 6 `[[injection]]` entries | 5 `Hooks` methods + 2 `BlockEntityPart` methods (table below) |
| `horizon-native.json` | gone (the manifest is the only one) |
| content `assets/`, `data/` inside a second `.hmod` | `assets/`, `data/` at the package root |
| — | `side = "both"` (a node is placed and ticked by an integrated server, which belongs to the client installation), `capabilities = ["registries"]` (fixed shared value) |

### 4. Old injection → new surface

| Old injection (helper) | New surface | Notes |
|---|---|---|
| `embedded_content.rs` (`content_root` HEAD, `native_data!`, `.horizon-cache`) | **gone** | one package carries `assets/` + `data/`; nothing is written to a cache and no dev list is appended |
| `block_entity.rs` (`block_entity`, HEAD, gave node **and** connector an entity) | `Hooks::register`: `registrar.block_entity_type("laser_node").blocks(&["laser_node"]).build()` + `registrar.block_entity_part(&kind, Node::new(queue))` | a declared type starts with an empty compound, so no `block_entity` hook is needed; the **connector deliberately has none** (its `facing` is a block state and it holds no data) |
| — | `BlockEntityPart::block_entity_ticks` → `event.ticks = true` | a Part method: the node ticks so its network can be found |
| — | `BlockEntityPart::block_entity_tick` → queue `event.pos` for the level in place | there is no way to enumerate a level's block entities, so 0.1.0's global scan became this queue |
| `level_tick.rs` (`level_tick_post` HEAD) | `Hooks::level_tick_post(host: HostSide, event: &mut LevelTickPostEvent)` | `event.level: LevelRef`; drain the queue, walk each network, one **transaction** per network |
| `right_click.rs` (`right_click_block` HEAD) | `Hooks::right_click_block(host: HostSide, event: &mut RightClickBlockEvent) -> Flow` | `event.click`: `pos`, `hit.face` (the `[i32;3]` normal, unchanged), `hand`, `stack`, `player`; `Flow::Cancel` takes the click |
| `register.rs` (`register_commands` HEAD) | `Hooks::register_commands` → `Command::new` / `Command::op(usage, 2)` | same six usages, same level 2 on the administration ones |
| `perform.rs` (`perform_command` HEAD) | `Hooks::run_mod_command` → `Flow` | `perform_command` no longer exists; the invocation carries parsed `literals` and typed `arguments` |
| `support.rs` (`Game<'_>` + `static`-free functions) | `Node { queue: Arc<Mutex<Queue>> }` + plain functions over `&mut dyn ServerHost` | the macro boxes one `Default` `Mod`; the Part gets a clone of the same `Arc` in `register` |

### 5. What changed in the glue's behaviour (not in `runtime.rs`)

- **Containers are the `ITEMS` capability**, not `native_container_slots`/`native_move_items`:
  `level.capability(host, &ITEMS, pos, Some(Direction::ALL[opposite(face)]))` gives a side-filtered
  `ResourceHandler<ItemResource>` (`size`/`resource`/`amount`/`extract`/`insert_any`). It answers
  `None` for a level that is not in place, which is why the pass belongs in `level_tick_post`.
- **One transaction per network per tick.** A card's move is: extract from the source slot, insert
  into the destination, and **put the refused remainder back into the source** — all inside the
  network's transaction, so a full target cannot lose an item and an aborted pass moves nothing.
  0.1.0 relied on `native_move_items` putting the item back.
- **Cards are written with `replace_data`**, the same transactional load and client update as
  `/data merge block`. Replacing (not merging) the payload is what makes `remove` really drop the
  key instead of leaving the old card behind; `set_changed`/chunk marking come with it, so the old
  `game.world.modified` insert is gone.
- **The node's tick no longer writes anything**: it queues. The network's pass runs once at the end
  of the level's tick, in game-time order, exactly as before.
- **The connector no longer has a block entity** (see the table above). Its model, its `facing`
  state and `/laserio face` are unchanged.
- **`/laserio` parsing is typed**: `info`/`card`/`remove`/`mode`/`rate`/`face` are registered
  usages, and the handlers read `pos: block_pos`, `side: word`, `mode: word`, `amount: integer`
  by name. The readout text is byte-for-byte 0.1.0's (`Laser Node at 1,100,0: targets 2,100,0
  east`), so the e2e's assertions still match.

### 6. Ladder measured for 0.2.0

Run in the port, `CARGO_INCREMENTAL=0`, each inside a `build-slot.sh run` lease (logs in
`ports/_shared/evidence/laserio/`):

| Step | Command | Result |
|---|---|---|
| Runtime unit tests | `cargo test` | **10 passed**, 0 failed (`sdk-test.log`, `sdk-ladder.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` (never `--all`: trap 4) | `fmt_exit=0` (`sdk-fmt.log`) |
| Package | `rm -f Cargo.lock; cargo horizon-mod build` | `target/horizon-mod/laserio.hmod`, EXIT 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check laserio.hmod` | `laserio 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **46,253 B**, sha256 `c84be8b854bde485bb8b030db42ddb36ed9a6d5810a716240c48438fdd32870f`, **EXIT 0** (`sdk-check.log`, `sdk-check-json.log`) |
| What it uses | the package's `uses.json` | **318** surface entries (`sdk-uses.json`), including `hook:block_entity_ticks`, `hook:block_entity_tick`, `hook:level_tick_post`, `hook:right_click_block`, `hook:register_commands`, `hook:run_mod_command`, `capability:horizon:items` |
| Fits the pinned build | `cargo horizon-mod fits laserio.hmod --build nightly` | `0b14f8a  made for`, EXIT 0 (`sdk-fits.log`) |
| Reproducible | `cargo horizon-mod build` again | byte-identical, same 46,253 B and sha256 |
| Fresh clone | `.horizon/`, `target/`, `Cargo.lock` removed, then `cargo horizon-mod build` | SDK fetched by itself; same size, same sha256 (`sdk-fresh-clone.log`) |
| `runtime.rs` untouched | `cmp` against the retired copy and `unzip -p … \| sha256sum` | `45627ed6…dce5` in all three places (`sdk-runtime-sha256.txt`) |
| Package contents | `unzip -l laserio.hmod` | 46 entries: 6 root documents (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `uses.json`), 3 `src/`, 24 `assets/`, 13 `data/`; **no `Cargo.lock`** |
| Skills line | `python3 ports/tools/check-skills.py --notes ports/laserio` | `ok laserio/notes.md: Skills line present` |

### 7. Traps, all hit here (1–15 are the reference's and were re-confirmed; 16–25 are this port's)

1. A leftover old path refuses the package (`unexpected package path: mixins/laserio/runtime.rs`):
   move `mixins/`, the `.hnmod` and `content/` **before** the first build.
2. A 38-character hash reads as missing pins — copy the full 40-hex commits from `sdk.json`; the
   mixins commit is `e59da90…`, not the old campaign pin.
3. `cargo horizon-mod check` with no file checks **wasm**; `check <file.hmod>` is the package check.
4. `cargo fmt --all` formats the SDK's crates through the path dependency; use `cargo fmt`.
5. A packed `Cargo.lock` changes the artifact: `rm -f Cargo.lock` right before the build, and keep
   `/Cargo.lock` in `.gitignore`.
6. `side` is the installation side: a node is ticked by an integrated server, so `"both"`.
7. `capabilities = ["registries"]` is mandatory even for a mod that registers nothing.
8. `runtime.rs` must be `pub mod runtime;` (a private module dead-codes every `pub` item).
9. `Hooks` is `Send + Sync`: the session state is a `Mutex`, and the Part shares it through an
   `Arc<Mutex<…>>` cloned in `register`.
10. `#[non_exhaustive]` enums need a wildcard arm.
11. The context types changed shape (`LifecycleContext { authority, epoch, tick }`, `get()`).
12. A `WorldOnly` host method is refused in the registration phase (do not move passes into
    `register`).
13. No `rust-toolchain.toml`: the pinned commit and the launcher decide the toolchain.
14. `cargo horizon-mod build` *does* pack a native project, contrary to the pinned `docs/mods.md`.
15. The in-game e2e needs a **launcher-built** game.
16. **`JavaString` is `From<&str>`, not `From<String>`.** `card_key(face).into()` on a `String`
    fails E0277; write `card_key(face).as_str().into()`. (`Compound`'s `get`/`insert` take a
    `JavaString`, so this bites every dynamic key — the six face keys are exactly that.)
17. **`cargo horizon-mod check --uses <file>` compiles the SDK instead of reading the file.**
    `--uses` *before* the positional file means "check the project, print what it uses", i.e. the
    wasm trap again (it went to crates.io and rebuilt the api crates) and it leaves a `Cargo.lock`
    behind — delete it before packaging. The uses list is the `uses.json` **inside** the package:
    `unzip -p laserio.hmod uses.json`.
18. **`clippy::too_many_arguments` is a design signal, not noise.** The card pass started at 8
    arguments; grouping the pass's invariants into `struct Pass<'a> { lasers, level, network }`
    fixed the lint and made the transaction body readable.
19. **`BlockClick.hit.face` is the `[i32; 3]` normal**, exactly what `face_of_normal` took from the
    old hook, so the face mapping carried over unchanged. `click.stack` is `Option<&ItemStack>` and
    `click.hand` says which hand — the old `native_held()` gave neither.
20. **A per-tick machine needs a block entity type declared in code**: `registrar.block_entity_type`
    `.blocks(&["laser_node"]).build()` plus `block_entity_part`, and its `block_entity_ticks` is
    where the type opts in. There is no data-file declaration.
21. **A level's block entities cannot be enumerated.** 0.1.0 scanned `game.world.block_entities`;
    the SDK has no equivalent, so the node's tick queues its position and `level_tick_post` drains
    it per level (`LevelRef` keyed), exactly as `integratedtunnels` learned.
22. **`perform_command` is gone** → `run_mod_command` with parsed `literals` and typed arguments;
    the args are read by name (`pos`, `side`, `mode`, `amount`), so the old free-text `split` of
    `/laserio` is gone with it.
23. **Containers are the `ITEMS` capability**, and the capability answers `None` outside the level
    in place — which is why the pass must stay in `level_tick_post` and must not be moved into the
    block entity tick (where the ticking entity is also out of the world).
24. **`replace_data` is the write for a card that can be removed.** `merge_data` cannot delete a
    key, so `remove` would leave the saved card behind.
25. **`cargo horizon-mod build` packs every root document and `src/`.** Rewrite `README.md` (it
    described `mixins/` and `content/`) before building and read `unzip -l` afterwards: a stale root
    file ships inside the package. `FINDINGS.md`, `notes.md`, `VALIDATION.md` and `tools/` are not
    packed.

### 8. Changelog / supersede text (for the resubmission that replaces #872)

> **0.2.0 — republished as a native `.hmod` (the SDK format).** Horizon's mod-API merge retired the
> `.hnmod` container, so this release replaces the closed submission #872 with one flat `.hmod`
> that carries the laser node, the laser connector, the item card, the logic chips, their models,
> textures, lang, recipes and loot **together with the code** — the embedded-content workaround of
> 0.1.0 (a second `.hmod` inside the bundle, written to `.horizon-cache` at start) is gone, and one
> file installs both halves. Built against SDK nightly `0b14f8a` (mixins `e59da90`, api 0.2, format
> native-1). Behaviour is unchanged: an extract item card moves its amount — one item at the shipped
> default, up to 64 after `/laserio rate` — out of the inventory on its side into the inventories its
> network reaches (a node's insert card sides and a laser connector's facing side) once every 20
> ticks, nothing is extracted unless a destination accepts it, and `/laserio info` reports the
> cards, the network, the targets and each card's amount and cadence. Every move of a network now
> shares one transaction per tick, so a full target can never lose an item. `src/runtime.rs` is
> byte-identical to 0.1.0 and keeps its 10 tests.

### 9. Gaps this migration adds to the slice's list

- **The in-game e2e was not re-run** (trap 15): a native package is compiled into the client by
  `horizon-launcher native-build <port> --instance <i> --trust-native-code`, and
  `horizon-launcher` is not installed in this environment. `tools/e2e.sh` keeps A–F's assertions,
  takes `GAME=<launcher-built client>` (no `--dev-mod`: the package carries the content) and an
  optional `CONTROL_GAME=<unmodded client of the same build>` for the negative control; G now
  asserts the package's structure instead of an embedded copy. The 0.1.0 run stands as the
  behaviour's evidence until that run is recorded.
- **`can_take` is always `true`** for the `ITEMS` handler, which is already side-filtered: a mod
  container that lists a slot through a side but refuses the extract makes that card idle for the
  tick (no fallback to the next slot). Narrower than 0.1.0's per-slot `can_take`; recorded rather
  than worked around.
- **The connector has no block entity** any more (0.1.0 gave it one through the `block_entity`
  hook). Nothing in this slice read it; a later slice that stores per-connector state (advanced
  pairing) must declare one.
- **The source item may leave and return within one operation** when a destination refuses part of
  the budget: the move is extract → insert → return-the-rest in one transaction. Observably the
  same as 0.1.0's `native_move_items`, but a comparator that samples mid-transaction would see it.
- **A command acts on the level in place.** `/laserio face` writes through
  `ServerHost::set_block_property`, which acts on the level the server ticks now; a command run in a
  parked dimension finds "no laser connector there" instead of writing the wrong level. Single
  player and a LAN host are unaffected; it is recorded rather than worked around.
- **`/laserio remove` on a side with no card says so** ("the east side has no card") instead of
  "card on the east side removed" as 0.1.0 did: the same state, an honest line.

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion (this migration — the recipe is fixed, so no design choice was open and `brainstorming` did not apply; the 0.1.0 record below also lists `brainstorming`); BeautifulRust/rust-best-practices,rust-port

## Verification (0.1.0, the retired `.hnmod`)

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every build, test
and e2e ran through `ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no bare
cargo, no `--target-dir`); the in-game run used the pin-matched shared
`horizon-ctl` (`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`) and the
port-built fast game, built and run in **one** e2e lease.

### Ladder

| Step | Command (through the slot runner) | Result |
|---|---|---|
| runtime RED | `cargo test --offline` (port) | `exit=101`, **75** `error[E…]` — the API did not exist yet (`evidence/laserio/runtime-red.log`) |
| runtime GREEN | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked --offline` | `fmt_exit=0`, `clippy_exit=0`, **10 passed**, 0 failed — `evidence/laserio/runtime-green.log` |
| stage | `python3 tools/horizon.py stage` | `stage_changed: true`, 151 s, the plan's **2 modules / 5 injections** accepted by the planner |
| check | `python3 tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** — `evidence/laserio/check.log` |
| embedded tests | `python3 tools/horizon.py test` (`--lib game::laserio::tests`) | **10 passed**, 0 failed, 1240 filtered — `evidence/laserio/test-embedded.log` |
| fast build | `python3 tools/horizon.py build` | `cargo_exit: 0` — `evidence/laserio/build.log` |
| content `.hmod` | `tools/package.sh` (`cargo-horizon-mod build` + `check`) | **19,421 B**, SHA-256 `3a86b149b2dcc126abfe98bf321fb03b2fa39516d16e4f89269c68ffba8ba219`, **38 entries**, `laserio 1.9.11 (mod, API 0.2, both)`, "no code: content only" |
| native `.hnmod` | `tools/package.sh` (`tools/package.py`) | **13,051 B**, SHA-256 `855c8aec5fc8435e76cbcea063650734bf9ec4a21c94603df874e144e5e87fe4`, **9 entries**; re-packaging reproduces the same bytes; `horizon-native.json` pins `horizon` `1d63e0a5…`, `mixins` `5a64af93…`, `game` 26.3 |
| content carry check | `python3 tools/carry_content.py --check` | `ok 38 files, 70436 bytes` — content/ is exactly the jar + port-owned files |
| e2e | `build-slot.sh run … --step e2e` — `tools/e2e.sh` | **PASS**, `EXIT:0` — `evidence/laserio/e2e.log` |
| selftest | `SELFTEST=1 tools/e2e.sh`, same lease | **`FAIL: SELFTEST: the target chest holds 11 stone, so the check is alive`**, `EXIT:1` — `evidence/laserio/e2e-selftest.log` |
| `.hmod` load check | `tools/hmod-load.sh` (shared game, `--dev-mod content/laserio.hmod`) | `PASS`: the package registers `laserio:laser_node`, `laserio:laser_connector` and `laserio:card_item`; `EXIT:0` — `evidence/laserio/e2e-hmod.log` |
| `.hmod` negative control | same script, second world **without** `--dev-mod` | `PASS`: `Unknown block type 'laserio:laser_node'`, `Unknown item 'laserio:card_item'`, block is air; `EXIT:0` (same log) |
| e2e log audit | `python3 ports/tools/check-e2e-logs.py laserio --strict` | `logs=4 ok=4 missing-exit=0 bad-exit=0` |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/laserio` | `ok laserio/notes.md: Skills line present` |
| submission gate | `python3 ports/tools/upstream_ready.py laserio` | **BLOCKED** at the split (the registry takes one file; see slice 1b below) |

### Slice 1b — one `.hnmod`: the embedded content (2026-11-10)

The registry stores **one `file` per version**, and the unified `.hmod` route is closed by
measurement: the game loads it, the registry index and the launcher refuse it. So the port
adopted the **AE2 pattern** (`ARCHITECTURE.md` §8 "Embedded content works", proved on
`charginggadgets`/PR #526): the built content mod travels **inside** the `.hnmod`.

- `mixins/laserio/data/laserio-content.hmod` — the built `content/laserio.hmod`,
  byte-identical (19,421 B, SHA-256 `3a86b149…`); it is a **build input** of the bundle, not a
  second deliverable.
- `mixins/laserio/embedded_content.rs` — one helper: `horizon_client::native_data!("laserio",
  "laserio-content.hmod")` compiles the file in; at start it is written to
  `.horizon-cache/laserio-embedded-content.hmod` when the copy on disk differs, the path is
  appended to the dev list, and `content_root(root, enabled, with)` is called. If the write
  fails the helper logs and returns `Continue`, so the game keeps its normal (contentless)
  load — that path is the control in the e2e.
- One `[[injection]]` (`id = "embedded-content"`, `kind = "HEAD"`, `cancellable = true`,
  `target = "horizon_client::game::mods::content_root"`, bindings `root`/`enabled`/`dev`) —
  the pin's documented raw-target exception for this hook.
- `tools/package.py` packs the bundle's `data/` folder as `mixins/<id>/data/…` (the pin's
  "Data files" rule) and **refuses** a `data/` copy that differs from `content/laserio.hmod`,
  or a missing one when the plan injects `content_root`. `tools/package.sh` rebuilds the
  `.hmod` first, `cmp`s it against the `data/` copy, packs the `.hnmod`, then runs
  `cargo horizon-mod check laserio.hnmod --kind native-mod`.

Nothing in the behaviour changed: same hooks, same runtime, same content; this slice only
moves the content inside the one file. The `.hnmod` grew from 13,051 B / 9 entries to
**28,783 B / 11 entries** (the embedded `.hmod` plus the helper).

### Ladder, slice 1b

| Step | Command (through the slot runner) | Result |
|---|---|---|
| crate ladder | `cargo fmt --check`, `clippy --all-targets --locked --offline -- -D warnings`, `cargo test --locked --offline` | `fmt_exit=0 clippy_exit=0 test_exit=0`, **10 passed** — `evidence/laserio/embed-ladder.log` |
| stage | `python3 tools/horizon.py stage` | `stage_changed: true`, the plan's **2 modules / 6 injections** accepted — `evidence/laserio/stage.log` |
| check | `python3 tools/horizon.py check` | `cargo_exit: 0` — `evidence/laserio/embed-check.log` |
| embedded tests | `python3 tools/horizon.py test` | `cargo_exit: 0` — `evidence/laserio/embed-test.log` |
| fast build | `python3 tools/horizon.py build` | `cargo_exit: 0` — `evidence/laserio/embed-build.log` |
| package | `tools/package.sh` (`.hmod` → `data/` → `.hnmod` → check) | `.hmod`/`data/` `3a86b149…` (38 entries), `.hnmod` **28,783 B** SHA-256 `eff433749b8a4ba2a0fd29ba09e4e715eab0141fb12a72c9bc8c912988fccd53`, **11 entries** — `evidence/laserio/embed-package.log` |
| native check | `cargo horizon-mod check laserio.hnmod --kind native-mod` | `laserio 0.1.0 (native-mod, format native-1/1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9/5a64af938ec86b00d53d16a408aabc0038fb3e57, for 26.3)` — same log |
| e2e (no `--dev-mod`) | `build-slot.sh e2e … -- tools/e2e.sh` (`EMBEDDED=1 CONTROL=1`) | **PASS**, `EXIT:0` — `evidence/laserio/e2e-embedded.log` |
| e2e control | same run: `.horizon-cache` is a plain file | `Unknown block type 'laserio:laser_node'`, `Unknown item 'laserio:card_item'`, block stayed `minecraft:air` |
| selftest | `SELFTEST=1 CONTROL=0 tools/e2e.sh` | `FAIL: SELFTEST: the target chest holds 10 stone, so the check is alive`, `EXIT:1` — `evidence/laserio/e2e-embedded-selftest.log` |
| e2e log audit | `python3 ports/tools/check-e2e-logs.py laserio --strict` | `logs=5 ok=5 missing-exit=0 bad-exit=0` |
| submission gate | `python3 ports/tools/upstream_ready.py laserio` | **READY** (exit 0) |

The pre-embedding runs are kept as `evidence/laserio/e2e-pre-embedding.log` and
`e2e-pre-embedding-selftest.log`; their scenario loaded `content/` with `--dev-mod`.
`tools/e2e.sh` now defaults to `EMBEDDED=1` (no `--dev-mod`, content from the bundle) with
`CONTROL=1`, and still accepts `EMBEDDED=0` for the old dev-mod path.

### In-game transcript, one file (quoted from `evidence/laserio/e2e-embedded.log`)

```
== control: the same game with no embedded content file ==
  launch: no --dev-mod; the bundle carries its own content
  setblock reply: {"chat":["Unknown block type 'laserio:laser_node'","...k 4 100 0 laserio:laser_node<--[HERE]"],"ok":true}
  give reply: {"chat":["Unknown item 'laserio:card_item'","give @s laserio:card_item 1<--[HERE]"],"ok":true}
  without the embedded file: no block, no item (block 4,100,0 = minecraft:air)
  embedded content: 19421 B at /tmp/opencode/laserio-home.jSnzll/.horizon-cache/laserio-embedded-content.hmod, identical to content/laserio.hmod
== A: a card moves 20 stone through the connector ==
  Laser Node at 1,100,0: west extract channel 0, 1 item(s) every 20 ticks
  Laser Node at 1,100,0: targets 2,100,0 east
  source before: 19, target before: 1
  9 operations in 8 s at 1 item each: source 10, target 10
  source after: 0, target after: 20
== A2: /laserio rate 4 moves four items per operation ==
  4 operations in 6 s at 4 items each: target 40
== G: one file — the .hnmod carries the content, and it is the only content mod ==
  /home/deepseek/atm10/ports/laserio/laserio.hnmod: 11 entries, mixins/laserio/data/laserio-content.hmod sha256 3a86b149…
  content mods in the run's home: /tmp/opencode/laserio-home.jSnzll/.horizon-cache/laserio-embedded-content.hmod
PASS: ... the content came from the embedded copy inside the one delivered .hnmod and was unknown when it could not be written
EXIT:0
```

That is the fail-capable pair this slice needs: **with** the one `.hnmod` installed and no
`--dev-mod` the blocks, the items and the transfer all work; **without** the embedded file
(write impossible) both ids are unknown, so the content demonstrably comes from inside the
bundle and not from anything else on disk.

The first e2e attempt failed on an over-strict assertion of mine — it read the
source chest *after* the card had already started, so it saw 18 + 2 rather than
20 + 0.  The transfer was working; the check now asserts the two chests hold 20
together (`evidence/laserio/first-attempt.log` keeps the run).

`tools/test_template.py`/`test_stage.py` from the scaffold are not this ladder;
they are the template's own fixtures. The staged check/test/build above is the
evidence (as `enderio`/`moderndynamics` record for the same scaffold).

### In-game scenario (quoted from `evidence/laserio/e2e.log`)

```
== A: a card moves 20 stone through the connector ==
  Laser Node at 1,100,0: west extract channel 0, 1 item(s) every 20 ticks
  Laser Node at 1,100,0: targets 2,100,0 east
  Laser Node at 1,100,0: network 1,100,0 2,100,0
  Laser Node at 1,100,0: cards west extract channel 0
  Laser Connector at 2,100,0: facing east
  source before: 18, target before: 2
  9 operations in 8 s at 1 item each: source 9, target 11
  source after: 0, target after: 20
== A2: /laserio rate 4 moves four items per operation ==
  Laser Node at 1,100,0: west extract channel 0, 4 item(s) every 20 ticks
  4 operations in 6 s at 4 items each: target 40
== B: a node without a card moves nothing (control) ==
  Laser Node at 8,100,0: cards none
  source: 4, target: 0
== C: a card whose network has no connector moves nothing (control) ==
  Laser Node at 14,100,0: targets none
  source: 4, target: 0
== D: a full target refuses the items (control) ==
  Laser Node at 24,100,0: targets 25,100,0 east
  source: 4, target stone: 0, target dirt: 1728
== E: the card, its mode and its rate survive a relaunch ==
  Laser Node at 1,100,0: west extract channel 0, 4 item(s) every 20 ticks
== F: right-clicking a node with an item card places it (upstream's path) ==
  Laser Node at 32,100,0: south insert channel 0, 1 item(s) every 20 ticks
== G: a launch without --dev-mod does not know the content (control) ==
  setblock reply: {"chat":["Unknown block type 'laserio:laser_node'","...k 4 100 0 laserio:laser_node<--[HERE]"],"ok":true}
  give reply: {"chat":["Unknown item 'laserio:card_item'","give @s laserio:card_item 1<--[HERE]"],"ok":true}
  block 4,100,0 = minecraft:air (control)
PASS: an extract card moved 20 stone through a connector to a chest, exactly one item at each 20-tick operation and exactly four after /laserio rate 4, the readout showed the card, the rate and the target, the card survived a relaunch, a right-click placed a card, and a cardless node / an unlinked connector / a full target / a world without --dev-mod moved or knew nothing
EXIT:0
```

What the scenario proves: 20 stone reaches the target through the connector and
every sampled step is **exactly one item** at **one operation per 20 ticks**
(9 operations in 8 s, not 80); after `/laserio rate 4` every step is **exactly
four**; the readout names the card, its side, mode, channel, rate and the
target; the settings survive a relaunch; a right-click with the card places it;
a node with no card (B), a card whose network has no connector (C) and a full
target (D) move nothing while the source keeps its items; and a world without
the content package has neither the block nor the item (G). `SELFTEST=1` flips
the phase-A assertion and the run fails with `EXIT:1`, so the check can fail.

### Harness notes for the next slice

The arena is The End at y=100 with a stone floor at y=99 (ambient item entities
do not exist there). A native command must be typed with `ctl chat "/cmd"` (the
`perform_command` hook); `ctl command` bypasses it. `ctl interact X Y Z FACE`
only aims at a face visible from the player's eye, which is why `/laserio card`
exists for a card on an occluded side and the right-click path is proved on a
free-standing node. `ctl wait N` is ticks. `ctl give` lands on the next tick.

## Review repairs (PR #584)

1. Staging is the orchestrator's (horizon-mods#686).
2. *"Fix the original-project link … Also repair the truncated description"* —
   the entry links `https://github.com/Direwolf20-MC/LaserIO`, credits
   Direwolf20-MC, and the description is rewritten complete.
3. **[P2] "Skip empty slots while looking for an extraction candidate"** —
   **fixed**: `first_item` now takes the first *non-empty* takeable stack, so an
   empty earlier slot cannot stall a network.
4. **[P1] "Restrict remote mutation commands or enforce survival/proximity
   rules"** — **fixed**: `card`, `remove`, `mode`, `rate` and `face` are
   operator-only (level 2); `info` stays open.
5. **[P2] "Ship the required MIT notices"** — `mixins/laserio/LICENSE` and
   `NOTICE.md` travel inside the `.hnmod`, and both sit at the embedded content
   archive's root.

### Verification after the repairs

- Artifact: `laserio.hnmod`, 31,547 B, SHA-256 `347d66e5c45b2c320d149ff92fb138427336f49950d9752570cadbbff2967fa9` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/laserio.hmod`).
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test --locked --offline`: clean, 10 passed; the embedded run (`python3 tools/horizon.py test --horizon ports/_shared/pins/horizon-1d63e0a`) is 10 passed, 0 failed (`target/horizon-test.log`).
- `build-slot.sh e2e --job … --port ports/laserio -- tools/e2e.sh`: PASS, log `ports/_shared/evidence/laserio/e2e-repair.log` ends `EXIT:0` (`check-e2e-logs.py laserio --strict` exit 0).
- `python3 ports/tools/upstream_ready.py laserio` and `check-skills.py --notes ports/laserio`: exit 0.
