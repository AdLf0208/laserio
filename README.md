# LaserIO → Horizon (0.2.2, the SDK package)

A Horizon port of **LaserIO 1.9.11** (`Direwolf20-MC/LaserIO`, **MIT**), from the
exact ATM10 pack jar `laserio-1.9.11.jar` (SHA-256 `03e8537d…9ef4df`, kept at
`../_shared/licence-cache/`).

**Slices 1–2:** the `laser_node` block with one item card per side, a
`laser_connector` in the node's network, and the container the connector or an
insert card faces. One item type moves per operation at upstream's default rate
(one item every 20 ticks); `/laserio rate` raises the amount to 64.
**Channels route:** an extract card reaches only the insert cards on its own
channel, a connector relays for every channel, and a channel no destination
serves extracts nothing; `/laserio channel` sets it. The five carried recipes and
both loot tables are proven in game. Energy and fluid cards, filters, stockers,
sensors, overclockers, the node menu and the wrench are later slices (see
`notes.md`).

## Layout (one flat `.hmod`, the SDK format of 2026-11-18)

| Path | What |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity (`id`, `version`, `api`, `side`, `[native]` pins) |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` |
| `src/laser.rs` | game glue: the node part, the tick queue, the network pass, the card placement, `/laserio` |
| `src/runtime.rs` | the pure card/network rules and their tests (with `laser.rs`: 18 tests) |
| `assets/laserio/`, `data/laserio/` | the carried content: node, connector, cards, models, textures, lang, recipes, loot |
| `data/c/tags/item/` | the `c:` tags the recipes name, declared against the vanilla items |
| `laserio.hmod` | **the delivered package** — content and code in one file |
| `tools/e2e.sh`, `tools/craft_check.py` | the in-game scenario (needs a launcher-built game, `--offscreen`) |
| `FINDINGS.md` | what the jar and upstream do, and what the slices implement |
| `notes.md`, `VALIDATION.md` | the port record and the ladder evidence |

## Commands

```sh
# pure runtime and glue (18 tests)
ports/tools/build-slot.sh run --job lio-test --port ports/laserio --step test -- \
    bash -c 'cd ports/laserio && cargo test'

# lints and formatting of *this* crate (never `cargo fmt --all`: it formats the SDK)
ports/tools/build-slot.sh run --job lio-lint --port ports/laserio --step lint -- \
    bash -c 'cd ports/laserio && cargo clippy --all-targets -- -D warnings'

# the package (no Cargo.lock: a packed lock changes the artifact) and its checks
ports/tools/build-slot.sh run --job lio-pack --port ports/laserio --step build -- \
    bash -c 'cd ports/laserio && rm -f Cargo.lock && cargo horizon-mod build &&
             cp target/horizon-mod/laserio.hmod ./laserio.hmod &&
             cargo horizon-mod check laserio.hmod &&
             cargo horizon-mod fits laserio.hmod --build 0a875da'

# the in-game check (a launcher-built game carries the mod: no --dev-mod)
eval "$(ports/tools/launcher-game.sh env laserio)"
ports/tools/build-slot.sh run --job lio-e2e --port ports/laserio --step e2e -- \
    bash ports/laserio/tools/e2e.sh
```

Local-only until the maintainer approves publishing (ARCHITECTURE.md §8); MIT
allows derivatives and redistributing this port with its licence and source.
