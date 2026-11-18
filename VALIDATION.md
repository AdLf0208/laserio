# VALIDATION — laserio (0.1.0 `.hnmod` history; the 0.2.0 SDK ladder is in `notes.md` §6)

> **Current format: the SDK package (0.2.0).** Every step below belongs to the **retired**
> `.hnmod` layout and its old pin; the 0.2.0 ladder — `cargo test` (10 passed), `clippy`
> (0 warnings), `fmt --check`, `cargo horizon-mod build` → `laserio.hmod` 46,244 B sha256
> `c84be8b8…870f`, `check` / `fits` EXIT 0, reproducible and fresh-clone identical — is recorded in
> [`notes.md`](notes.md) §6, with its logs in `ports/_shared/evidence/laserio/sdk-*.log`.

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every step ran
through `ports/tools/build-slot.sh` (or `remote-slot.sh`, which wraps it) with
`CARGO_INCREMENTAL=0`; no bare cargo, no `--target-dir`; the e2e rebuilt and ran
in **one** lease.

## Slice 1b — one `.hnmod` (the AE2 embedding)

| Step | Evidence file | Result |
|---|---|---|
| crate ladder | `evidence/laserio/embed-ladder.log` | `fmt_exit=0 clippy_exit=0 test_exit=0`, 10 tests passed |
| stage | `evidence/laserio/stage.log` | `stage_changed: true`, 2 modules / 6 injections |
| check | `evidence/laserio/embed-check.log` | `cargo_exit: 0` |
| embedded tests | `evidence/laserio/embed-test.log` | `cargo_exit: 0` |
| fast build | `evidence/laserio/embed-build.log` | `cargo_exit: 0` |
| packages | `evidence/laserio/embed-package.log` | `.hmod` + `data/` copy 19,421 B `3a86b149…ba219` 38 entries · `.hnmod` **28,783 B `eff43374…cd53` 11 entries** (carries `mixins/laserio/data/laserio-content.hmod`) |
| native check | `evidence/laserio/embed-package.log` | `cargo horizon-mod check laserio.hnmod --kind native-mod` → `laserio 0.1.0 (native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3)` |
| e2e, **no `--dev-mod`** | `evidence/laserio/e2e-embedded.log` | **PASS** + `EXIT:0`; the control first (`.horizon-cache` a plain file) shows both ids unknown |
| selftest | `evidence/laserio/e2e-embedded-selftest.log` | **FAIL** (`target chest holds 10 stone`) + `EXIT:1` |
| log audit | `check-e2e-logs.py laserio --strict` | `logs=5 ok=5 missing-exit=0 bad-exit=0` |
| key assertions | `e2e-embedded.log` | content written from the bundle (19,421 B, identical) and **the only content mod in the home**; 9 ops / 8 s, each step **+1**; rate 4 → each step **+4**; source 0, target 20/40 |

## Slice 1 (pre-embedding record)

| Step | Evidence file | Result |
|---|---|---|
| runtime RED | `evidence/laserio/runtime-red.log` | exit 101, 75 compile errors |
| runtime GREEN | `evidence/laserio/runtime-green.log` | fmt 0 · clippy 0 warnings · 10 tests passed |
| ladder | `evidence/laserio/ladder.log` | stage 0 · check 0 · embedded test 0 · build 0 · package 0 |
| check | `evidence/laserio/check.log` | `cargo_exit: 0`, 0 warnings |
| embedded tests | `evidence/laserio/test-embedded.log` | 10 passed, 0 failed, 1240 filtered |
| fast build | `evidence/laserio/build.log` | `cargo_exit: 0` |
| packages | `evidence/laserio/package.log` | `.hmod` 19,421 B `3a86b149…ba219` 38 entries · `.hnmod` 13,051 B `855c8aec…7fe4` 9 entries |
| e2e (`--dev-mod content/`) | `evidence/laserio/e2e-pre-embedding.log` | PASS + `EXIT:0` |
| selftest | `evidence/laserio/e2e-pre-embedding-selftest.log` | FAIL (`target chest holds 11 stone`) + `EXIT:1` |
| `.hmod` load + negative control | `evidence/laserio/e2e-hmod.log` | PASS + `EXIT:0` both legs |

Reproducibility: `python3 tools/package.py` re-run gives the same `.hnmod`
SHA-256; `cargo-horizon-mod build` twice gives the same `.hmod` bytes;
`tools/carry_content.py --check` re-derives all 38 content files from the jar.

Submission: `upstream_ready.py laserio` is **READY** (exit 0) — one `.hnmod`
carries the content and the code, so the registry's one-file rule is met. The
ledger status stays the orchestrator's to set.
