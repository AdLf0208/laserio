#!/usr/bin/env python3
"""Craft the recipes LaserIO actually carries, in a real crafting table.

Called by `tools/e2e.sh` with the open game's control socket. Uses the harness's
crafting grid (`ctl screen`: slots 100-108 row-major, result 200), gives the
ingredients and asserts, for each carried shaped recipe, the result's **id and
count**, that the grid was **consumed**, and that a pattern the recipe does not
declare crafts nothing (the control: the grid is live, so an empty result is the
recipe's answer).

The four shaped recipes (`data/laserio/recipe/`), in dependency order:

1. `logic_chip_raw.json`  "rgr" / "cqc" / "rgr"  -> 4 x `laserio:logic_chip_raw`
2. `laser_connector.json` " g " / "rbr" / "iii"  -> 1 x `laserio:laser_connector`
3. `laser_node.json`      "igi" / "gbg" / "igi"  -> 1 x `laserio:laser_node`
4. `card_item.json`       "rlr" / "qpq" / "ggg"  -> 1 x `laserio:card_item`

The keys are the mod's `c:` tags (`c:nuggets/gold`, `c:gems/lapis`,
`c:gems/quartz`, `c:dusts/redstone`, `c:ingots/iron`, `c:glass_blocks`,
`c:glass_panes`), which Horizon does not provide — the port declares them in
`data/c/tags/item/` so the recipes can match at all. The smelting recipe
(`logic_chip.json`, 100 ticks) is checked by the caller in a furnace.

The table is replaced before every case, so an untaken result cannot leak into
the next grid. Exits 1 with the observed result on the first mismatch.
"""
import json
import subprocess
import sys

# `data/laserio/recipe/*.json`, row-major over slots 100..108.
CASES = [
    {
        "label": "logic_chip_raw.json: 4 redstone + 2 gold + 2 clay + 1 quartz block",
        "items": [
            ("minecraft:redstone", 4),
            ("minecraft:gold_nugget", 2),
            ("minecraft:clay_ball", 2),
            ("minecraft:quartz_block", 1),
        ],
        "place": [
            ("minecraft:redstone", ["100", "102", "106", "108"]),
            ("minecraft:gold_nugget", ["101", "107"]),
            ("minecraft:clay_ball", ["103", "105"]),
            ("minecraft:quartz_block", ["104"]),
        ],
        "want": ("laserio:logic_chip_raw", 4),
        "consume": [
            "minecraft:redstone",
            "minecraft:gold_nugget",
            "minecraft:clay_ball",
            "minecraft:quartz_block",
        ],
    },
    {
        "label": "laser_connector.json: 1 glass + 2 redstone + 1 chip + 3 iron",
        "items": [
            ("minecraft:glass", 1),
            ("minecraft:redstone", 2),
            ("laserio:logic_chip", 1),
            ("minecraft:iron_ingot", 3),
        ],
        "place": [
            ("minecraft:glass", ["101"]),
            ("minecraft:redstone", ["103", "105"]),
            ("laserio:logic_chip", ["104"]),
            ("minecraft:iron_ingot", ["106", "107", "108"]),
        ],
        "want": ("laserio:laser_connector", 1),
        "consume": [
            "minecraft:glass",
            "minecraft:redstone",
            "laserio:logic_chip",
            "minecraft:iron_ingot",
        ],
    },
    {
        "label": "laser_node.json: 4 iron + 4 glass panes + 1 connector",
        "items": [
            ("minecraft:iron_ingot", 4),
            ("minecraft:glass_pane", 4),
            ("laserio:laser_connector", 1),
        ],
        "place": [
            ("minecraft:iron_ingot", ["100", "102", "106", "108"]),
            ("minecraft:glass_pane", ["101", "103", "105", "107"]),
            ("laserio:laser_connector", ["104"]),
        ],
        "want": ("laserio:laser_node", 1),
        "consume": [
            "minecraft:iron_ingot",
            "minecraft:glass_pane",
            "laserio:laser_connector",
        ],
    },
    {
        "label": "card_item.json: 2 redstone + 1 lapis + 2 quartz + 1 chip + 3 gold",
        "items": [
            ("minecraft:redstone", 2),
            ("minecraft:lapis_lazuli", 1),
            ("minecraft:quartz", 2),
            ("laserio:logic_chip", 1),
            ("minecraft:gold_nugget", 3),
        ],
        "place": [
            ("minecraft:redstone", ["100", "102"]),
            ("minecraft:lapis_lazuli", ["101"]),
            ("minecraft:quartz", ["103", "105"]),
            ("laserio:logic_chip", ["104"]),
            ("minecraft:gold_nugget", ["106", "107", "108"]),
        ],
        "want": ("laserio:card_item", 1),
        "consume": [
            "minecraft:redstone",
            "minecraft:lapis_lazuli",
            "minecraft:quartz",
            "laserio:logic_chip",
            "minecraft:gold_nugget",
        ],
    },
]


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        # A bounded reply: a `ctl` call with no timeout blocks forever, and the
        # kill then looks like a pass (ports/_shared/launcher-e2e.md, trap 8).
        out = subprocess.run(
            [ctl, "--socket", sock, "--timeout", "30", *args],
            text=True,
            capture_output=True,
        )
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(ticks=4):
        run("wait", str(ticks))

    def slots():
        screen = run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def find(item):
        for slot in slots():
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the inventory")

    def result():
        """The result slot's stack, or None: an empty slot has `item: null`."""
        stack = [s for s in slots() if s["slot"] == 200]
        if not stack or not stack[0].get("item"):
            return None
        return stack[0]

    def place(item, targets):
        slot = find(item)
        run("click-slot", str(slot))
        wait(1)
        for target in targets:
            run("click-slot", str(target), "right")
            wait(1)
        run("click-slot", str(slot))
        wait(2)

    def setup(items):
        try:
            run("close")
        except SystemExit:
            pass
        wait(5)
        # Replace the table so its block entity, and therefore the crafting
        # grid, is fresh for this case.
        run("setblock", x, y, z, "minecraft:air")
        run("setblock", x, y, z, "minecraft:crafting_table")
        wait(2)
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, count in items:
            run("give", item, str(count))
        wait(3)
        run("interact", x, y, z)
        wait(5)
        # A menu is a client screen, so it can take a tick to appear; poll the
        # screen rather than assuming the first reply has slots.
        screen = {}
        for _ in range(10):
            screen = run("screen")
            if "slots" in screen:
                return
            wait(2)
        raise SystemExit(f"the crafting table did not open a menu: {screen}")

    def counts():
        return run("inventory").get("counts", {})

    failures = []

    def expect(case):
        setup(case["items"])
        for item, targets in case["place"]:
            place(item, targets)
        got = result()
        print(f"{case['label']} ->", json.dumps(got))
        want_item, want_count = case["want"]
        if not got or got.get("item") != want_item or got.get("count") != want_count:
            failures.append(f"{case['label']}: expected {want_count} {want_item}, got {got}")
            return
        run("click-slot", "200", "shift")
        wait(3)
        run("close")
        wait(3)
        got_counts = counts()
        print("  inventory after taking it ->", json.dumps(got_counts))
        if got_counts.get(want_item, 0) != want_count:
            failures.append(
                f"{case['label']}: the inventory holds {got_counts.get(want_item, 0)} "
                f"{want_item}, expected {want_count}"
            )
        for item in case["consume"]:
            if got_counts.get(item, 0) != 0:
                failures.append(
                    f"{case['label']}: {got_counts.get(item, 0)} {item} left, "
                    "so the grid was not consumed"
                )

    def expect_empty(label, items, placements):
        setup(items)
        for item, targets in placements:
            place(item, targets)
        got = result()
        print(f"{label} ->", json.dumps(got))
        if got is not None:
            failures.append(f"{label}: control crafted {got.get('count')} {got.get('item')}")

    for case in CASES:
        expect(case)

    # The control: the connector's ingredients in a pattern no recipe declares
    # (the chip moved off the centre) must craft nothing.
    expect_empty(
        "control: the connector's ingredients off their pattern",
        [
            ("minecraft:glass", 1),
            ("minecraft:redstone", 2),
            ("laserio:logic_chip", 1),
            ("minecraft:iron_ingot", 3),
        ],
        [
            ("minecraft:glass", ["101"]),
            ("minecraft:redstone", ["103", "105"]),
            ("laserio:logic_chip", ["100"]),
            ("minecraft:iron_ingot", ["106", "107", "108"]),
        ],
    )

    run("close")
    wait(3)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)
    print(
        "recipe PASS: the four carried shaped recipes craft their declared results "
        "(4 logic_chip_raw, 1 laser_connector, 1 laser_node, 1 card_item) and consume "
        "their ingredients, from the c: tags the port declares, and the same "
        "ingredients in a pattern no recipe declares craft nothing"
    )


if __name__ == "__main__":
    main()
