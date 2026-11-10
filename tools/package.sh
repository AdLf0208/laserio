#!/usr/bin/env bash
# Package both artifacts, in the order the embedded-content bundle needs:
#  - the .hmod via the pin's own cargo-horizon-mod (build + check), from content/
#  - copy it to mixins/laserio/data/laserio-content.hmod (the AE2 pattern: the
#    content travels inside the one .hnmod the registry stores)
#  - the source-only .hnmod (tools/package.py, deterministic bytes), which
#    refuses a data/ copy that differs from the built content/laserio.hmod
# Run inside a build slot (it compiles the packaging tool):
#   ports/tools/build-slot.sh run --job lio-pack --port ports/laserio \
#       --step package -- env CARGO_INCREMENTAL=0 tools/package.sh
set -euo pipefail

PORT=$(cd "$(dirname "$0")/.." && pwd)
HORIZON=${HORIZON:-${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}/ports/_shared/pins/horizon-1d63e0a}
LOGS=$PORT/target/ladder
mkdir -p "$LOGS"
cd "$PORT"

CARGO_TARGET_DIR=$PORT/target/carry-tools
export CARGO_TARGET_DIR
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod build) \
    2>&1 | tee "$LOGS/package-hmod.log"
cp content/target/horizon-mod/laserio.hmod content/laserio.hmod
(cd content && cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- horizon-mod check laserio.hmod) \
    2>&1 | tee -a "$LOGS/package-hmod.log"

# The bundle's copy is the built content, never a stale one.
cmp content/laserio.hmod mixins/laserio/data/laserio-content.hmod \
    || { echo "mixins/laserio/data/laserio-content.hmod is not the built content/laserio.hmod:" >&2
         sha256sum content/laserio.hmod mixins/laserio/data/laserio-content.hmod >&2
         echo "copy content/laserio.hmod into mixins/laserio/data/laserio-content.hmod" >&2
         exit 1; }

python3 tools/package.py | tee "$LOGS/package-hnmod.log"

# The bundle the registry stores must pass its own checks: `.hnmod`, native code,
# with `mixins/laserio/data/laserio-content.hmod` inside.
CARGO_TARGET_DIR=$PORT/target/carry-tools cargo run --locked --offline --quiet \
    --manifest-path "$HORIZON/crates/cargo-horizon-mod/Cargo.toml" -- \
    horizon-mod check laserio.hnmod --kind native-mod 2>&1 | tee -a "$LOGS/package-hnmod.log"

echo "== artifacts =="
for f in content/laserio.hmod mixins/laserio/data/laserio-content.hmod laserio.hnmod; do
    [ -f "$f" ] || { echo "missing $f"; exit 1; }
    size=$(stat -c %s "$f")
    sha=$(sha256sum "$f" | cut -d' ' -f1)
    entries=$(python3 -c "import zipfile,sys; print(len(zipfile.ZipFile(sys.argv[1]).namelist()))" "$f")
    echo "$f size=$size sha256=$sha entries=$entries"
done
