# LaserIO → Horizon (slice 1, the SDK package)

A Horizon port of **LaserIO 1.9.11** (`Direwolf20-MC/LaserIO`, **MIT**), from the
exact ATM10 pack jar `laserio-1.9.11.jar` (SHA-256 `03e8537d…9ef4df`, kept at
`../_shared/licence-cache/`).

**Slice 1:** the `laser_node` block with one item card per side, a
`laser_connector` in the node's network, and the container the connector or an
insert card faces. One item type moves per operation at upstream's default rate
(one item every 20 ticks); `/laserio rate` raises the amount to 64. Energy and
fluid cards, channels, filters, stockers, sensors, overclockers, the node menu and
the wrench are later slices (see `notes.md`).

## Layout (one flat `.hmod`, the SDK format of 2026-11-18)

| Path | What |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity (`id`, `version`, `api`, `side`, `[native]` pins) |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` |
| `src/laser.rs` | game glue: the node part, the tick queue, the network pass, the card placement, `/laserio` |
| `src/runtime.rs` | the pure card/network rules and their tests — **byte-identical** to 0.1.0, do not edit |
| `assets/laserio/`, `data/laserio/` | the carried content: node, connector, cards, models, textures, lang, recipes, loot |
| `laserio.hmod` | **the delivered package** — content and code in one file |
| `tools/e2e.sh` | the in-game scenario (needs a launcher-built game) |
| `FINDINGS.md` | what the jar and upstream do, and what this slice implements |
| `notes.md`, `VALIDATION.md` | the port record and the ladder evidence |

## Commands

```sh
# pure runtime (the 10 tests)
ports/tools/build-slot.sh run --job lio-test --port ports/laserio --step test -- \
    bash -c 'cd ports/laserio && cargo test'

# lints and formatting of *this* crate (never `cargo fmt --all`: it formats the SDK)
ports/tools/build-slot.sh run --job lio-lint --port ports/laserio --step lint -- \
    bash -c 'cd ports/laserio && cargo clippy --all-targets -- -D warnings'

# the package (no Cargo.lock: a packed lock changes the artifact) and its checks
ports/tools/build-slot.sh run --job lio-pack --port ports/laserio --step build -- \
    bash -c 'cd ports/laserio && rm -f Cargo.lock && cargo horizon-mod build &&
             cp target/horizon-mod/laserio.hmod ./laserio.hmod &&
             cargo horizon-mod check laserio.hmod && cargo horizon-mod fits laserio.hmod --build nightly'

# the in-game check (a launcher-built game carries the mod: no --dev-mod)
GAME=<launcher-built horizon-client> tools/e2e.sh
```

Local-only until the maintainer approves publishing (ARCHITECTURE.md §8); MIT
allows derivatives and redistributing this port with its licence and source.
