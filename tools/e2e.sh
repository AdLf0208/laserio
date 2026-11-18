#!/usr/bin/env bash
# In-game check for LaserIO slice 1: one laser node moving items through an
# adjacent laser connector, at the card/connector rate.
#
# The End arena (stone floor at y=99, everything at y=100): an empty level keeps
# ambient item entities out of the container counts.
#
#  A  chest (0) - node (1) - connector (2) - chest (3), an EXTRACT item card on
#     the node's west side and the connector facing east: the 20 stone of the
#     source chest move one item per 20-tick operation; after `/laserio rate …
#     4` the same card moves four per operation. Every sampled step is exactly
#     the declared amount.
#  B  chest (7) - node (8) - connector (9) - chest (10), NO card: nothing moves
#     (control: a node without a card is inert).
#  C  chest (13) - node (14) .. gap .. connector (16) - chest (17), an EXTRACT
#     card on the node: nothing moves (control: a connector outside the node's
#     network is not a destination).
#  D  chest (23) - node (24) - connector (25) - chest (26, full of dirt), a card
#     on the source side: nothing moves and the source keeps its items
#     (control: a refused item stays in its slot).
#  E  a relaunch keeps the card and its rate with the node.
#  F  right-clicking a node with an item card places it on the clicked side.
#  G  one package: the delivered `.hmod` carries the node, the connector, the
#     cards and their assets/data beside the code, and nothing is written to a
#     `.horizon-cache` dev file any more.
#
# How it is launched (the SDK format, 2026-11-18): a native mod is compiled into
# the client by the launcher, so this script takes
#   GAME=<launcher-built horizon-client carrying laserio>
# and launches it with **no `--dev-mod`**: the content (`assets/`, `data/`) is
# inside the same package as the code. `CONTROL_GAME=<an unmodded client of the
# same build>` is the negative control — both ids must be unknown there, so the
# blocks and the transfer demonstrably come from this package. Without it the
# control is skipped and the log says so; SELFTEST=1 (which inverts A's first
# assertion, so the run must fail) is still the fail-capable check of the
# behaviour itself.
#
# The in-game run was NOT re-done in the SDK migration: `horizon-launcher` is not
# installed in this environment (see `notes.md`). The 0.1.0 run stands as the
# behaviour's evidence; this script keeps every assertion for the first run on a
# launcher-built game.
#
# SELFTEST=1 flips A's first assertion (the target must still be empty), so the
# scenario must FAIL; that proves the check can fail.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the mod's command hook.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/laserio
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
CONTROL_GAME=${CONTROL_GAME:-}
HMOD=${HMOD:-$PORT/laserio.hmod}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/laserio-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=LaserIOE2E
CONTROL_WORLD=LaserIOControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "missing GAME: point it at a launcher-built horizon-client that carries laserio" >&2
    echo "  horizon-launcher native-build $PORT --instance <i> --trust-native-code" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/laserio-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

# count X Y Z ITEM: how many of ITEM the container at X,Y,Z holds.
count() {
    ctl container "$1" "$2" "$3" \
        | jq -r --arg i "$4" '[.slots[] | select(.item == $i) | .count] | add // 0'
}

# The `/laserio info` line for a node, echoed and matched by TEXT.
expect_info() { # expect_info X Y Z TEXT
    say "laserio info $1 $2 $3"
    ctl wait 3 >/dev/null
    lines=$(ctl messages 8 | jq -r '.chat[]' | grep "at $1,$2,$3:" | head -5)
    echo "$lines" | sed 's/^/  /'
    echo "$lines" | grep -q -- "$4" || fail "expected /laserio info to say '$4'"
}

# Sample the target chest's count every half second for SECONDS and return the
# number of changes, failing unless every change is exactly AMOUNT items.
sample_operations() { # sample_operations X Y Z ITEM SECONDS AMOUNT
    local before after step i steps=0
    before=$(count "$1" "$2" "$3" "$4")
    step=$(( $5 * 2 ))
    for i in $(seq 1 "$step"); do
        sleep 0.5
        after=$(count "$1" "$2" "$3" "$4")
        if [ "$after" != "$before" ]; then
            [ $(( after - before )) -eq "$6" ] \
                || fail "one operation moved $(( after - before )) items, not $6"
            steps=$(( steps + 1 ))
            before=$after
        fi
    done
    echo "$steps"
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [extra args]
    rm -f "$SOCK"
    cd "$SCRATCH"
    local game=${LAUNCH_GAME:-$GAME}
    local args=(--game "$game" --world "${WORLD_NAME:-$WORLD}" --seed 20261105 --creative
                --headless --wait --resources "$RES")
    echo "  launch: $(basename "$game"), no --dev-mod: the package carries its content" >&2
    "$CTL" --socket "$SOCK" launch "${args[@]}" "$@" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 40
}

# A stone floor at y=99 in the End with clear air above, the player on it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s 0.5 100 3.5"
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5
        if [ "$(ctl block 0 99 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill -8 99 -8 44 99 8 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 100 -8 44 108 8 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

block_at() { # block_at X Z BLOCK
    ctl command "execute in minecraft:the_end run setblock $1 100 $2 $3"
}

# The negative control: an unmodded client of the same build. Both ids must be
# unknown there, so the blocks, the items and the transfer demonstrably come from
# this package and not from anything else on disk. Skipped (and said so) when no
# `CONTROL_GAME` is given.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "== control: skipped (set CONTROL_GAME=<unmodded client> to run it) =="
        return
    fi
    [ -x "$CONTROL_GAME" ] || fail "missing CONTROL_GAME: $CONTROL_GAME"
    echo "== control: the same build with no laserio =="
    LAUNCH_GAME=$CONTROL_GAME WORLD_NAME=$CONTROL_WORLD launch
    arena
    setblock=$(ctl command "setblock 4 100 0 laserio:laser_node")
    echo "  setblock reply: $setblock"
    case "$setblock" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew laserio:laser_node, so the content did not come from this package" ;;
    esac
    give=$(ctl command "give @s laserio:card_item 1")
    echo "  give reply: $give"
    case "$give" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew laserio:card_item, so the content did not come from this package" ;;
    esac
    name=$(ctl block 4 100 0 | jq -r '.state // .name // "unknown"')
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    echo "  without the package: no block, no item (block 4,100,0 = $name)"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (The End arena; one package, code and content together) =="
control_leg
launch
arena

echo "== build the nodes, connectors and chests =="
block_at 0 0 minecraft:chest
block_at 1 0 laserio:laser_node
block_at 2 0 laserio:laser_connector
block_at 3 0 minecraft:chest

block_at 7 0 minecraft:chest
block_at 8 0 laserio:laser_node
block_at 9 0 laserio:laser_connector
block_at 10 0 minecraft:chest

block_at 13 0 minecraft:chest
block_at 14 0 laserio:laser_node
block_at 16 0 laserio:laser_connector
block_at 17 0 minecraft:chest

block_at 23 0 minecraft:chest
block_at 24 0 laserio:laser_node
block_at 25 0 laserio:laser_connector
block_at 26 0 minecraft:chest

block_at 32 0 laserio:laser_node
ctl wait 6

node=$(ctl block 1 100 0 | jq -r '.state // .name // "unknown"')
echo "  node at 1,100,0: $node"
case "$node" in
    laserio:laser_node*) ;;
    *) fail "the laser node did not register: $node" ;;
esac
connector=$(ctl block 2 100 0 | jq -r '.state // .name // "unknown"')
echo "  connector at 2,100,0: $connector"
case "$connector" in
    laserio:laser_connector*) ;;
    *) fail "the laser connector did not register: $connector" ;;
esac

ctl container 0 100 0 minecraft:stone 20 >/dev/null
ctl container 7 100 0 minecraft:stone 4 >/dev/null
ctl container 13 100 0 minecraft:stone 4 >/dev/null
ctl container 23 100 0 minecraft:stone 4 >/dev/null
for _ in $(seq 1 27); do ctl container 26 100 0 minecraft:dirt 64 >/dev/null; done
[ "$(ctl container 26 100 0 | jq '[.slots[] | select(. == null)] | length')" = 0 ] \
    || fail "the full target chest is not full"
ctl wait 4

echo "== A: a card moves 20 stone through the connector =="
say "laserio card 1 100 0 west"
ctl wait 2 >/dev/null
say "laserio mode 1 100 0 west extract"
ctl wait 2 >/dev/null
say "laserio face 2 100 0 east"
ctl wait 2 >/dev/null
expect_info 1 100 0 "west extract channel 0"
expect_info 1 100 0 "1 item(s) every 20 ticks"
expect_info 1 100 0 "targets 2,100,0 east"
expect_info 2 100 0 "facing east"
echo "  source before: $(count 0 100 0 minecraft:stone), target before: $(count 3 100 0 minecraft:stone)"
[ $(( $(count 0 100 0 minecraft:stone) + $(count 3 100 0 minecraft:stone) )) = 20 ] \
    || fail "the source and target chests together did not take 20 stone"

steps=$(sample_operations 3 100 0 minecraft:stone 8 1)
echo "  $steps operations in 8 s at 1 item each: source $(count 0 100 0 minecraft:stone), target $(count 3 100 0 minecraft:stone)"
if [ "$SELFTEST" = 1 ]; then
    [ "$(count 3 100 0 minecraft:stone)" = 0 ] \
        || fail "SELFTEST: the target chest holds $(count 3 100 0 minecraft:stone) stone, so the check is alive"
else
    [ "$steps" -ge 5 ] || fail "only $steps operation(s) in 8 s, expected 5 to 9"
    [ "$steps" -le 9 ] || fail "$steps operations in 8 s: the card is not one per 20 ticks"
fi
for _ in $(seq 1 60); do
    [ "$(count 3 100 0 minecraft:stone)" = 20 ] && break
    sleep 0.5
done
echo "  source after: $(count 0 100 0 minecraft:stone), target after: $(count 3 100 0 minecraft:stone)"
[ "$(count 0 100 0 minecraft:stone)" = 0 ] || fail "the source chest is not empty"
[ "$(count 3 100 0 minecraft:stone)" = 20 ] || fail "the target chest did not receive all 20 stone"

echo "== A2: /laserio rate 4 moves four items per operation =="
ctl container 0 100 0 minecraft:stone 20 >/dev/null
ctl wait 2
say "laserio rate 1 100 0 west 4"
ctl wait 2 >/dev/null
expect_info 1 100 0 "4 item(s) every 20 ticks"
steps=$(sample_operations 3 100 0 minecraft:stone 6 4)
echo "  $steps operations in 6 s at 4 items each: target $(count 3 100 0 minecraft:stone)"
[ "$steps" -ge 3 ] || fail "only $steps operation(s) in 6 s, expected 3 to 6"
[ "$steps" -le 6 ] || fail "$steps operations in 6 s: the card is not one per 20 ticks"
for _ in $(seq 1 60); do
    [ "$(count 3 100 0 minecraft:stone)" = 40 ] && break
    sleep 0.5
done
[ "$(count 0 100 0 minecraft:stone)" = 0 ] || fail "the source chest is not empty at rate 4"
[ "$(count 3 100 0 minecraft:stone)" = 40 ] || fail "the target chest did not receive 40 stone at rate 4"

echo "== B: a node without a card moves nothing (control) =="
say "laserio face 9 100 0 east"
ctl wait 3 >/dev/null
ctl wait 80 >/dev/null
expect_info 8 100 0 "cards none"
echo "  source: $(count 7 100 0 minecraft:stone), target: $(count 10 100 0 minecraft:stone)"
[ "$(count 7 100 0 minecraft:stone)" = 4 ] || fail "the cardless source lost stone"
[ "$(count 10 100 0 minecraft:stone)" = 0 ] || fail "stone moved without a card"

echo "== C: a card whose network has no connector moves nothing (control) =="
say "laserio card 14 100 0 west"
say "laserio mode 14 100 0 west extract"
say "laserio face 16 100 0 east"
ctl wait 3 >/dev/null
expect_info 14 100 0 "targets none"
ctl wait 80 >/dev/null
echo "  source: $(count 13 100 0 minecraft:stone), target: $(count 17 100 0 minecraft:stone)"
[ "$(count 13 100 0 minecraft:stone)" = 4 ] || fail "the unlinked source lost stone"
[ "$(count 17 100 0 minecraft:stone)" = 0 ] || fail "stone moved to an unlinked connector"

echo "== D: a full target refuses the items (control) =="
say "laserio card 24 100 0 west"
say "laserio mode 24 100 0 west extract"
say "laserio face 25 100 0 east"
ctl wait 3 >/dev/null
expect_info 24 100 0 "targets 25,100,0 east"
ctl wait 80 >/dev/null
echo "  source: $(count 23 100 0 minecraft:stone), target stone: $(count 26 100 0 minecraft:stone), target dirt: $(count 26 100 0 minecraft:dirt)"
[ "$(count 23 100 0 minecraft:stone)" = 4 ] || fail "the source lost stone into a full target"
[ "$(count 26 100 0 minecraft:stone)" = 0 ] || fail "the full target took stone"
[ "$(count 26 100 0 minecraft:dirt)" = 1728 ] || fail "the full target lost its dirt"

echo "== D2: an empty earlier slot does not stall the network =="
# A source chest whose slot 0 is empty and slot 1 holds the stone: the first
# item type is in slot 1, so the transfer must still run.
block_at 33 0 minecraft:chest
block_at 34 0 laserio:laser_node
block_at 35 0 laserio:laser_connector
block_at 36 0 minecraft:chest
ctl wait 6
ctl container 33 100 0 minecraft:stone 4 >/dev/null
ctl command "execute in minecraft:the_end run item replace block 33 100 0 container.0 with minecraft:stone 4" >/dev/null
ctl command "execute in minecraft:the_end run item replace block 33 100 0 container.1 with minecraft:air" >/dev/null
ctl wait 4
[ "$(count 33 100 0 minecraft:stone)" = 4 ] || fail "the source chest's slot 0 was not filled"
# Move the four stone to slot 1, leaving slot 0 empty.
ctl command "execute in minecraft:the_end run item replace block 33 100 0 container.1 from block 33 100 0 container.0" >/dev/null
ctl command "execute in minecraft:the_end run item replace block 33 100 0 container.0 with minecraft:air" >/dev/null
ctl wait 4
say "laserio card 34 100 0 west"
say "laserio mode 34 100 0 west extract"
say "laserio face 35 100 0 east"
ctl wait 3 >/dev/null
expect_info 34 100 0 "targets 35,100,0 east"
for _ in $(seq 1 60); do
    [ "$(count 36 100 0 minecraft:stone)" = 4 ] && break
    sleep 0.5
done
echo "  source: $(count 33 100 0 minecraft:stone), target: $(count 36 100 0 minecraft:stone)"
[ "$(count 36 100 0 minecraft:stone)" = 4 ] \
    || fail "an empty first slot stalled the network: the target got $(count 36 100 0 minecraft:stone)"

echo "== E: the card, its mode and its rate survive a relaunch =="
ctl quit >/dev/null
sleep 2
launch
expect_info 1 100 0 "west extract channel 0"
expect_info 1 100 0 "4 item(s) every 20 ticks"
expect_info 1 100 0 "targets 2,100,0 east"

echo "== F: right-clicking a node with an item card places it (upstream's path) =="
ctl tp 32.5 100 3.5 >/dev/null
ctl give laserio:card_item 1 >/dev/null
ctl wait 4
if ! ctl select laserio:card_item >/dev/null 2>&1; then
    slot=$(ctl inventory | jq -r '(.hotbar // []) | to_entries[] | select((.value.item // "") == "laserio:card_item") | (.key + 1)' | head -1)
    [ -n "$slot" ] || fail "no item card in the player's hotbar"
    ctl select "$slot" >/dev/null
fi
ctl interact 32 100 0 south >/dev/null || fail "the right-click on the node at 32,100,0 failed"
ctl wait 3 >/dev/null
expect_info 32 100 0 "south insert channel 0"

echo "== G: one package — the .hmod carries the content and the code =="
python3 - "$HMOD" "$PORT/src/runtime.rs" <<'PY'
import hashlib
import sys
import zipfile

package, runtime = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(package) as archive:
    names = archive.namelist()
    assets = [n for n in names if n.startswith('assets/laserio/')]
    data = [n for n in names if n.startswith('data/laserio/')]
    for wanted in ('horizon-mod.toml', 'Cargo.toml', 'src/lib.rs', 'src/laser.rs',
                   'src/runtime.rs', 'LICENSE', 'LICENSE-NOTICE.md'):
        if wanted not in names:
            raise SystemExit(f'  FAIL: {package} does not carry {wanted}')
    for carried, kind in ((assets, 'assets'), (data, 'data')):
        if not carried:
            raise SystemExit(f'  FAIL: {package} carries no {kind}/laserio entries')
    packed = hashlib.sha256(archive.read('src/runtime.rs')).hexdigest()
    on_disk = hashlib.sha256(open(runtime, 'rb').read()).hexdigest()
    if packed != on_disk:
        raise SystemExit(f'  FAIL: the packed runtime.rs is {packed}, the port holds {on_disk}')
    print(f'  {package}: {len(names)} entries, {len(assets)} assets, {len(data)} data, '
          f'src/runtime.rs sha256 {packed}')
PY
[ -f "$HMOD" ] || fail "no delivered package at $HMOD"
if find "$SCRATCH" -name '*.hmod' -print -quit | grep -q .; then
    fail "the run's home holds a stray .hmod; this package's content must come from the installed mod"
fi
echo "  the run's home holds no dev-mod content file"

echo "PASS: an extract card moved 20 stone through a connector to a chest, an empty earlier source slot did not stall it, exactly one item at each 20-tick operation and exactly four after /laserio rate 4, the readout showed the card, the rate and the target, the card survived a relaunch, a right-click placed a card, a cardless node / an unlinked connector / a full target moved nothing, and the one delivered .hmod carries the content beside the code (an unmodded client of the same build knows neither id)"
