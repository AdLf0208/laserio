#!/usr/bin/env python3
"""Carry LaserIO's content from the pack jar into `content/`, deterministically.

The jar is read-only evidence: `ports/_shared/licence-cache/laserio-1.9.11.jar`
(MIT, SHA-256 `03e8537d…`). This script copies the assets/data this slice uses
verbatim, and writes the 26.3-shaped files this port owns (item definitions,
block definitions, creative tab, loot/recipe fixes, a trimmed lang file).

    python3 tools/carry_content.py           # write content/
    python3 tools/carry_content.py --check   # fail if content/ differs
"""
import argparse
import hashlib
import json
import sys
import zipfile
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
JAR = PORT.parent / "_shared/licence-cache/laserio-1.9.11.jar"
CONTENT = PORT / "content"
NS = "laserio"

# Files carried byte-for-byte from the jar.
CARRIED = [
    "assets/laserio/blockstates/laser_node.json",
    "assets/laserio/blockstates/laser_connector.json",
    "assets/laserio/models/block/laser_node.json",
    "assets/laserio/models/block/laser_connector.json",
    "assets/laserio/models/item/laser_node.json",
    "assets/laserio/models/item/laser_connector.json",
    "assets/laserio/models/item/logic_chip.json",
    "assets/laserio/models/item/logic_chip_raw.json",
    "assets/laserio/textures/block/laser_node.png",
    "assets/laserio/textures/block/laser_node.png.mcmeta",
    "assets/laserio/textures/block/laser_connector.png",
    "assets/laserio/textures/item/card_item.png",
    "assets/laserio/textures/item/cardlayers/channel.png",
    "assets/laserio/textures/item/cardlayers/extract.png",
    "assets/laserio/textures/item/cardlayers/insert.png",
    "assets/laserio/textures/item/logic_chip.png",
    "assets/laserio/textures/item/logic_chip_raw.png",
    "data/laserio/loot_table/blocks/laser_node.json",
    "data/laserio/loot_table/blocks/laser_connector.json",
    "data/laserio/recipe/card_item.json",
    "data/laserio/recipe/laser_connector.json",
    "data/laserio/recipe/laser_node.json",
    "data/laserio/recipe/logic_chip.json",
    "data/laserio/recipe/logic_chip_raw.json",
]


def self_drop_loot(item):
    """A block's self-drop in the 26.3 shape: the pool's *singular typed*
    condition (`"condition": {"type": …}`); the 1.21.1 plural form evaluates
    false and the block drops nothing."""
    return {
        "type": "minecraft:block",
        "pools": [
            {
                "bonus_rolls": 0.0,
                "condition": {"type": "minecraft:survives_explosion"},
                "entries": [{"type": "minecraft:item", "name": item}],
                "rolls": 1.0,
            }
        ],
        "random_sequence": f"laserio:blocks/{item.split(':')[1]}",
    }


# A six-faced block whose outline is the model's box.
def block(template, state=None, default=None, shapes=None):
    document = {
        "template": template,
        "properties": {
            "hardness": 2.0,
            "requires_tool": False,
            "sounds": "minecraft:stone",
            "occludes": False,
            "render_shape": "MODEL",
        },
        "shapes": shapes or {"default": [[0.1875, 0.1875, 0.1875, 0.8125, 0.8125, 0.8125]]},
        "item": True,
    }
    if state:
        document["state_properties"] = state
    if default:
        document["default_state"] = default
    return document


OWNED = {
    "horizon-mod.toml": """# Port-owned content mod for the LaserIO 1.9.11 slice: the laser node, the laser
# connector and the item card, their models, textures, lang, recipes and loot.
# Models, textures and recipes are upstream's own MIT assets, carried from the
# exact ATM10 jar (see ../../_shared/carry-work/laserio/SOURCE.md). The card and
# transfer behaviour is the native half in ports/laserio/mixins.
id = "laserio"
name = "LaserIO"
version = "1.9.11"
description = "Laser nodes and connectors moving items on a channel (slice 1)."
authors = ["Direwolf20", "ErrorMikey"]
api = "0.2"
side = "both"
capabilities = ["registries"]
""",
    "assets/laserio/items/laser_node.json": json.dumps(
        {"model": {"type": "minecraft:model", "model": "laserio:item/laser_node"}}, indent=2
    ) + "\n",
    "assets/laserio/items/laser_connector.json": json.dumps(
        {"model": {"type": "minecraft:model", "model": "laserio:item/laser_connector"}}, indent=2
    ) + "\n",
    "assets/laserio/items/card_item.json": json.dumps(
        {"model": {"type": "minecraft:model", "model": "laserio:item/card_item"}}, indent=2
    ) + "\n",
    "assets/laserio/items/logic_chip.json": json.dumps(
        {"model": {"type": "minecraft:model", "model": "laserio:item/logic_chip"}}, indent=2
    ) + "\n",
    "assets/laserio/items/logic_chip_raw.json": json.dumps(
        {"model": {"type": "minecraft:model", "model": "laserio:item/logic_chip_raw"}}, indent=2
    ) + "\n",
    # The jar's card model is carried without its mode overrides: the
    # `laserio:mode` item property is not registered here, so those models are
    # unreachable (the card's mode lives in the node's block entity instead).
    "assets/laserio/models/item/card_item.json": json.dumps(
        {
            "parent": "minecraft:item/generated",
            "textures": {
                "layer0": "laserio:item/card_item",
                "layer1": "laserio:item/cardlayers/insert",
                "layer2": "laserio:item/cardlayers/channel",
            },
        },
        indent=2,
    ) + "\n",
    "data/laserio/horizon/block/laser_node.json": json.dumps(block("minecraft:stone"), indent=2) + "\n",
    "data/laserio/loot_table/blocks/laser_node.json": json.dumps(
        self_drop_loot("laserio:laser_node"), indent=2
    ) + "\n",
    "data/laserio/loot_table/blocks/laser_connector.json": json.dumps(
        self_drop_loot("laserio:laser_connector"), indent=2
    ) + "\n",
    "data/laserio/horizon/block/laser_connector.json": json.dumps(
        block(
            "minecraft:stone",
            state={
                "facing": {
                    "type": "enum",
                    "values": ["down", "up", "north", "south", "west", "east"],
                }
            },
            default={"facing": "north"},
            shapes={"default": [[0.25, 0.25, 0.25, 0.75, 0.75, 0.75]]},
        ),
        indent=2,
    ) + "\n",
    "data/laserio/horizon/item/card_item.json": json.dumps(
        {"template": "minecraft:paper"}, indent=2
    ) + "\n",
    "data/laserio/horizon/item/logic_chip.json": json.dumps(
        {"template": "minecraft:paper"}, indent=2
    ) + "\n",
    "data/laserio/horizon/item/logic_chip_raw.json": json.dumps(
        {"template": "minecraft:paper"}, indent=2
    ) + "\n",
    "data/laserio/horizon/creative_tab/laserio.json": json.dumps(
        {
            "title": "LaserIO",
            "icon": "laserio:laser_node",
            "items": [
                "laserio:laser_node",
                "laserio:laser_connector",
                "laserio:card_item",
                "laserio:logic_chip",
                "laserio:logic_chip_raw",
            ],
        },
        indent=2,
    ) + "\n",
    "assets/laserio/lang/en_us.json": json.dumps(
        {
            "block.laserio.laser_node": "Laser Node",
            "block.laserio.laser_connector": "Laser Connector",
            "item.laserio.card_item": "Item Card",
            "item.laserio.logic_chip": "Logic Chip",
            "item.laserio.logic_chip_raw": "Raw Logic Chip",
            "itemGroup.laserio": "LaserIO",
        },
        indent=2,
    ) + "\n",
}


def files():
    """Every (relative path, bytes) the content half should hold."""
    if not JAR.is_file():
        raise SystemExit(f"missing pack jar: {JAR}")
    out = {}
    with zipfile.ZipFile(JAR) as jar:
        for name in CARRIED:
            out[name] = jar.read(name)
    for name, text in OWNED.items():
        out[name] = text.encode()
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if content/ differs")
    args = parser.parse_args()
    expected = files()
    if args.check:
        bad = []
        for name, data in sorted(expected.items()):
            path = CONTENT / name
            if not path.is_file() or path.read_bytes() != data:
                bad.append(name)
        present = {p.relative_to(CONTENT).as_posix() for p in CONTENT.rglob("*") if p.is_file()}
        extra = sorted(present - set(expected) - {"laserio.hmod"})
        for name in bad:
            print(f"differs: {name}")
        for name in extra:
            print(f"unexpected: {name}")
        if bad or extra:
            return 1
        print(f"ok {len(expected)} files, {sum(len(d) for d in expected.values())} bytes")
        return 0
    for name, data in sorted(expected.items()):
        path = CONTENT / name
        path.parent.mkdir(parents=True, exist_ok=True)
        # Deterministic rewrite: skip identical files so mtimes stay stable.
        if path.is_file() and path.read_bytes() == data:
            continue
        path.write_bytes(data)
    print(json.dumps({"files": len(expected), "jar_sha256":
                      hashlib.sha256(JAR.read_bytes()).hexdigest()}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
