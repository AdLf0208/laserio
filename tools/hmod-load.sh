#!/usr/bin/env bash
# Proves the built content package (content/laserio.hmod) loads on a real game:
# the shared, unmodded 1d63e0a game with `--dev-mod <the .hmod>` must register
# the laser node, the connector and the item card. A second launch WITHOUT
# `--dev-mod` is the negative control: the same ids must be unknown there, so
# the check cannot pass vacuously.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PIN_BIN/horizon-client}
HMOD=${HMOD:-$CLONE/ports/laserio/content/laserio.hmod}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/laserio-hmod.sock}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing package: $HMOD" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/laserio-hmod-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "${KEEP:-0}" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH" 2>/dev/null || true; fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch WORLD [--dev-mod PATH]
    local world=$1; shift
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "$@" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 20
}

held() { # held ITEM
    ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'
}

echo "== launch WITH the package (--dev-mod $HMOD) =="
launch LaserIOHmod --dev-mod "$HMOD"
ctl command "setblock 0 100 0 laserio:laser_node" >/dev/null
ctl command "setblock 1 100 0 laserio:laser_connector" >/dev/null
ctl wait 4
node=$(ctl block 0 100 0 | jq -r '.state // .name // empty')
connector=$(ctl block 1 100 0 | jq -r '.state // .name // empty')
echo "block 0,100,0 = $node"
echo "block 1,100,0 = $connector"
case "$node" in laserio:laser_node*) ;; *) fail "the package did not register the laser node" ;; esac
case "$connector" in laserio:laser_connector*) ;; *) fail "the package did not register the laser connector" ;; esac
ctl give laserio:card_item 1 >/dev/null
ctl wait 4
card=$(held laserio:card_item)
echo "card_item in the inventory: ${card:-0}"
[ "${card:-0}" != 0 ] || fail "the package did not register the item card"
echo "PASS: the packaged .hmod loaded (laser_node, laser_connector, card_item)"

echo "== launch WITHOUT the package (negative control) =="
ctl quit >/dev/null
sleep 2
launch LaserIOHmodNegative
setblock=$(ctl command "setblock 0 100 0 laserio:laser_node")
echo "setblock reply: $setblock"
case "$setblock" in
    *"Unknown block"*|*"unknown block"*|*"error"*) ;;
    *) fail "without --dev-mod the laser node was still known: $setblock" ;;
esac
give=$(ctl command "give @s laserio:card_item 1")
echo "give reply: $give"
case "$give" in
    *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
    *) fail "without --dev-mod the item card was still known: $give" ;;
esac
echo "PASS: without --dev-mod the laser node and the item card are absent"
