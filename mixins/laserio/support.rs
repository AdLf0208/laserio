// Game-side glue for LaserIO's laser node and connector (injected into
// `crate::game` as `laserio_state`; it needs the game, so it is not part of the
// standalone library).
//
// Upstream a laser node keeps cards per side and discovers its network through
// wrench links; an extract card hands a stack's worth of items to an insert
// card in the same channel. This slice keeps the cards in the node's block
// entity (one per side, encoded as [`Card::saved`]), treats the contiguous run
// of touching laser blocks as the network, and after the level's tick runs each
// extract card's 20-tick operation into the network's inventories.

use std::collections::BTreeSet;

use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::laserio::{
    Card, Cards, Endpoint, FACE_COUNT, FACE_NAMES, Face, OPERATION_TICK_DELAY, Pos, TransferMode,
    endpoint_order, face_named, face_normal, is_operation_tick, opposite, phase_of, spend_budget,
    step,
};
use crate::game::native_hooks::flags;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The laser node block (and its item).
pub(super) const NODE: &str = "laserio:laser_node";
/// The connector block (and its item).
pub(super) const CONNECTOR: &str = "laserio:laser_connector";
/// The item card.
pub(super) const CARD: &str = "laserio:card_item";

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&key(name)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The settings key one side saves its card under.
fn card_key(face: Face) -> String {
    format!("card_{}", FACE_NAMES[face])
}

/// A node's cards; a fresh node has none.
fn read_cards(nbt: &Compound) -> Cards {
    let mut cards = Cards::new();
    for face in 0..FACE_COUNT {
        if let Some(saved) = int(nbt, &card_key(face)) {
            cards = cards.with(face, Some(Card::from_saved(saved)));
        }
    }
    cards
}

fn write_cards(nbt: &mut Compound, cards: Cards) {
    for face in 0..FACE_COUNT {
        let key = key(&card_key(face));
        match cards.get(face) {
            Some(card) => {
                nbt.insert(key, Tag::Int(card.saved()));
            }
            None => {
                nbt.remove(&key);
            }
        }
    }
}

/// A placed laser block gets a block entity; a fresh one holds no cards, which
/// [`read_cards`] answers for an empty compound.
pub(super) fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if (block == NODE || block == CONNECTOR) && entity.is_none() {
        *entity = Some((block.to_string(), Compound::new()));
    }
}

/// The laser block entity at `pos`, if it is ours.
fn laser_at<'a>(game: &'a crate::game::Game<'_>, pos: Pos) -> Option<&'a ModBlockEntity> {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if entity.id == NODE || entity.id == CONNECTOR => {
            Some(entity)
        }
        _ => None,
    }
}

/// Whether the block at `pos` is a node or a connector.
fn is_laser(game: &crate::game::Game<'_>, pos: Pos) -> bool {
    let block = game.native_block(pos);
    block == NODE || block == CONNECTOR
}

/// Every laser block in the level, in a stable order.
fn laser_positions(game: &crate::game::Game<'_>) -> Vec<Pos> {
    let mut positions: Vec<Pos> = game
        .world
        .block_entities
        .iter()
        .filter_map(|(pos, entity)| match entity {
            BlockEntity::Mod(entity) if entity.id == NODE || entity.id == CONNECTOR => {
                Some([pos.0, pos.1, pos.2])
            }
            _ => None,
        })
        .collect();
    positions.sort_unstable();
    positions
}

/// The network a laser block belongs to: every node and connector reachable
/// through touching laser blocks (upstream walks wrench links instead).
fn network_of(game: &crate::game::Game<'_>, start: Pos) -> Vec<Pos> {
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    let mut queue = vec![start];
    seen.insert(start);
    while let Some(pos) = queue.pop() {
        for face in 0..FACE_COUNT {
            let beside = step(pos, face);
            if is_laser(game, beside) && seen.insert(beside) {
                queue.push(beside);
            }
        }
    }
    seen.into_iter().collect()
}

/// Whether the block at `pos` reaches an item container through `face`.
fn has_container(
    game: &crate::game::Game<'_>,
    dimension: &str,
    pos: Pos,
    face: Face,
) -> bool {
    game.native_container_slots(dimension, pos, Direction::ALL[face])
        .is_some_and(|slots| !slots.is_empty())
}

/// The side a connector points at, from its `facing` state.
fn connector_face(game: &crate::game::Game<'_>, pos: Pos) -> Option<Face> {
    game.native_block_property(pos, "facing")
        .and_then(|value| face_named(&value))
}

/// The inventories a network can insert into: a node's INSERT cards and a
/// connector's `facing` side, when that side reaches a container.
fn endpoints(
    game: &crate::game::Game<'_>,
    dimension: &str,
    network: &[Pos],
) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    for &laser in network {
        let faces: Vec<Face> = if game.native_block(laser) == CONNECTOR {
            connector_face(game, laser).into_iter().collect()
        } else {
            laser_at(game, laser)
                .map(|entity| read_cards(&entity.nbt))
                .map(|cards| {
                    cards
                        .cards()
                        .filter(|(_, card)| card.mode() == TransferMode::Insert)
                        .map(|(face, _)| face)
                        .collect()
                })
                .unwrap_or_default()
        };
        for face in faces {
            if has_container(game, dimension, step(laser, face), opposite(face)) {
                endpoints.push(Endpoint { laser, face });
            }
        }
    }
    endpoints
}

/// The item in the first slot the source side offers, one item type per
/// operation as upstream's `sendItems` takes the first valid stack.
fn first_item(
    game: &crate::game::Game<'_>,
    dimension: &str,
    pos: Pos,
    face: Face,
) -> Option<String> {
    let slots = game.native_container_slots(dimension, pos, Direction::ALL[face])?;
    slots
        .iter()
        .find(|slot| slot.can_take)
        .and_then(|slot| slot.stack.as_ref())
        .map(|stack| stack.item.to_string())
}

/// Run one extract card of `network` for this tick: move up to its amount into
/// the network's inventories, and nothing at all when there is no destination
/// (upstream extracts only what an inserter accepts).
fn run_card(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    network: &[Pos],
    laser: Pos,
    face: Face,
    card: Card,
) -> u32 {
    let source = Endpoint { laser, face };
    let from = step(laser, face);
    let from_face = opposite(face);
    let Some(item) = first_item(game, dimension, from, from_face) else {
        return 0;
    };
    let all = endpoints(game, dimension, network);
    let order = endpoint_order(&all, source);
    if order.is_empty() {
        return 0;
    }
    spend_budget(card.amount(), &order, |index, remaining| {
        let destination = all[index];
        let to = step(destination.laser, destination.face);
        let to_face = opposite(destination.face);
        game.native_move_items(
            dimension,
            from,
            Direction::ALL[from_face],
            to,
            Direction::ALL[to_face],
            Some(&item),
            remaining,
        )
    })
}

/// After the level's tick: run every extract card whose 20-tick operation falls
/// on this tick, once per node/connector network.
pub(super) fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let lasers = laser_positions(game);
    if lasers.is_empty() {
        return;
    }
    let game_time = game.world.game_time.max(0) as u64;
    let mut done: BTreeSet<Pos> = BTreeSet::new();
    let mut handled: BTreeSet<(Pos, Face)> = BTreeSet::new();
    for pos in lasers {
        if done.contains(&pos) {
            continue;
        }
        let network = network_of(game, pos);
        for &laser in &network {
            if game.native_block(laser) != NODE {
                continue;
            }
            let cards = laser_at(game, laser)
                .map(|entity| read_cards(&entity.nbt))
                .unwrap_or_default();
            for (face, card) in cards.cards() {
                if card.mode() != TransferMode::Extract
                    || !handled.insert((laser, face))
                    || !is_operation_tick(game_time, phase_of(laser, face))
                {
                    continue;
                }
                run_card(game, dimension, &network, laser, face, card);
            }
        }
        done.extend(network);
    }
}

/// The face a clicked normal names, if it names one.
fn face_of_normal(normal: [i32; 3]) -> Option<Face> {
    (0..FACE_COUNT).find(|&face| face_normal(face) == normal)
}

/// Put `card` on one side of the node at `pos`. Answers false when the block is
/// not a node or the side already holds a card.
fn set_card(game: &mut crate::game::Game<'_>, pos: Pos, face: Face, card: Option<Card>) -> bool {
    if game.native_block(pos) != NODE {
        return false;
    }
    let Some(current) = laser_at(game, pos).map(|entity| read_cards(&entity.nbt)) else {
        return false;
    };
    if card.is_some() && current.get(face).is_some() {
        return false;
    }
    let cards = current.with(face, card);
    if let Some(BlockEntity::Mod(entity)) =
        game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
    {
        if entity.id == NODE {
            write_cards(&mut entity.nbt, cards);
        }
    }
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    true
}

/// Replace the card on a side with an edited copy.
fn edit_card(
    game: &mut crate::game::Game<'_>,
    pos: Pos,
    face: Face,
    edit: impl FnOnce(Card) -> Card,
) -> bool {
    if game.native_block(pos) != NODE {
        return false;
    }
    let Some(current) = laser_at(game, pos).map(|entity| read_cards(&entity.nbt)) else {
        return false;
    };
    let Some(card) = current.get(face) else {
        return false;
    };
    let cards = current.with(face, Some(edit(card)));
    if let Some(BlockEntity::Mod(entity)) =
        game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
    {
        if entity.id == NODE {
            write_cards(&mut entity.nbt, cards);
        }
    }
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    true
}

/// Right-clicking a node with an item card places it on the clicked side, as
/// upstream's `LaserNode.useWithoutItem` does; the item is consumed outside
/// creative. A side that already carries a card is left for the (unported) menu.
pub(super) fn on_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: Pos,
    face_normal: [i32; 3],
    canceled: &mut bool,
) {
    if game.native_block(pos) != NODE {
        return;
    }
    let Some((held, _, _)) = game.native_held() else {
        return;
    };
    if held != CARD {
        return;
    }
    let Some(face) = face_of_normal(face_normal) else {
        return;
    };
    if !set_card(game, pos, face, Some(Card::new())) {
        *canceled = true;
        game.native_feedback(Text::literal(format!(
            "LaserIO: this side already has a card ({})",
            FACE_NAMES[face]
        )));
        return;
    }
    // Creative players keep their item, as upstream's `stack.shrink` guard does.
    if game.native_acting_player().is_none_or(|player| player.game_mode != 1) {
        game.native_take_items(CARD, 1);
    }
    *canceled = true;
    game.native_feedback(Text::literal(format!(
        "LaserIO: item card placed on the {} side",
        FACE_NAMES[face]
    )));
}

/// Declare `/laserio info|card|remove|mode|rate|face`.
pub(super) fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("laserio info <pos:block_pos>"));
    commands.push(NativeCommand::new("laserio card <pos:block_pos> <side:word>"));
    commands.push(NativeCommand::new(
        "laserio remove <pos:block_pos> <side:word>",
    ));
    commands.push(NativeCommand::new(
        "laserio mode <pos:block_pos> <side:word> <mode:word>",
    ));
    commands.push(NativeCommand::new(
        "laserio rate <pos:block_pos> <side:word> <amount:integer>",
    ));
    commands.push(NativeCommand::new(
        "laserio face <pos:block_pos> <side:word>",
    ));
}

fn parse_pos(words: &[&str; 3]) -> Option<Pos> {
    Some([
        words[0].parse().ok()?,
        words[1].parse().ok()?,
        words[2].parse().ok()?,
    ])
}

/// The cards a node holds, as `west extract channel 0, 1 item every 20 ticks`
/// or `none`.
fn card_list(cards: Cards) -> String {
    if cards.is_empty() {
        return "none".to_string();
    }
    let names: Vec<String> = cards
        .cards()
        .map(|(face, card)| {
            format!(
                "{} {} channel {}",
                FACE_NAMES[face],
                card.mode().name(),
                card.channel()
            )
        })
        .collect();
    names.join("; ")
}

/// The inventories a network reaches, as `2,100,0 east`.
fn target_list(endpoints: &[Endpoint]) -> String {
    let names: Vec<String> = endpoints
        .iter()
        .map(|endpoint| {
            format!(
                "{},{},{} {}",
                endpoint.laser[0], endpoint.laser[1], endpoint.laser[2], FACE_NAMES[endpoint.face]
            )
        })
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join("; ")
    }
}

fn parse_side(side: &str) -> Option<Face> {
    face_named(&side.to_lowercase())
}

/// Run one `/laserio` command: the harness readout and settings path (upstream's
/// node screen is a gap).
pub(super) fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    let dimension = game.native_dimension();
    match words.as_slice() {
        ["laserio", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /laserio info <x> <y> <z>"));
                return;
            };
            let Some((is_connector, cards)) = laser_at(game, pos)
                .map(|entity| (entity.id == CONNECTOR, read_cards(&entity.nbt)))
            else {
                game.native_feedback(Text::literal("LaserIO: no laser node or connector there."));
                return;
            };
            if is_connector {
                let facing = connector_face(game, pos)
                    .map(|face| FACE_NAMES[face])
                    .unwrap_or("none");
                game.native_feedback(Text::literal(format!(
                    "Laser Connector at {x},{y},{z}: facing {facing}"
                )));
            } else {
                game.native_feedback(Text::literal(format!(
                    "Laser Node at {x},{y},{z}: cards {}",
                    card_list(cards)
                )));
            }
            let network = network_of(game, pos);
            let members: Vec<String> = network
                .iter()
                .map(|member| format!("{},{},{}", member[0], member[1], member[2]))
                .collect();
            game.native_feedback(Text::literal(format!(
                "Laser Node at {x},{y},{z}: network {}",
                if members.is_empty() {
                    "none".to_string()
                } else {
                    members.join(" ")
                }
            )));
            let targets = target_list(&endpoints(game, dimension, &network));
            game.native_feedback(Text::literal(format!(
                "Laser Node at {x},{y},{z}: targets {targets}"
            )));
            for (face, card) in cards.cards() {
                game.native_feedback(Text::literal(format!(
                    "Laser Node at {x},{y},{z}: {} {} channel {}, {} item(s) every {} ticks",
                    FACE_NAMES[face],
                    card.mode().name(),
                    card.channel(),
                    card.amount(),
                    OPERATION_TICK_DELAY
                )));
            }
        }
        ["laserio", action @ ("card" | "remove"), x, y, z, side] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /laserio card|remove <x> <y> <z> <side>",
                ));
                return;
            };
            let Some(face) = parse_side(side) else {
                game.native_feedback(Text::literal(format!(
                    "Unknown side '{side}': down, up, north, south, west or east."
                )));
                return;
            };
            let card = (*action == "card").then(Card::new);
            if !set_card(game, pos, face, card) {
                game.native_feedback(Text::literal(
                    "LaserIO: no node there, or that side already has a card.",
                ));
                return;
            }
            game.native_feedback(Text::literal(format!(
                "LaserIO: card on the {} side {}",
                FACE_NAMES[face],
                if card.is_some() { "placed" } else { "removed" }
            )));
        }
        ["laserio", "mode", x, y, z, side, mode] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /laserio mode <x> <y> <z> <side> <mode>"));
                return;
            };
            let Some(face) = parse_side(side) else {
                game.native_feedback(Text::literal(format!("Unknown side '{side}'.")));
                return;
            };
            let Some(mode) = TransferMode::named(&mode.to_lowercase()) else {
                game.native_feedback(Text::literal("Unknown mode: insert or extract."));
                return;
            };
            if !edit_card(game, pos, face, |card| card.with_mode(mode)) {
                game.native_feedback(Text::literal("LaserIO: no card on that side."));
                return;
            }
            game.native_feedback(Text::literal(format!(
                "LaserIO: {} side is now {}",
                FACE_NAMES[face],
                mode.name()
            )));
        }
        ["laserio", "rate", x, y, z, side, amount] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /laserio rate <x> <y> <z> <side> <amount>",
                ));
                return;
            };
            let Some(face) = parse_side(side) else {
                game.native_feedback(Text::literal(format!("Unknown side '{side}'.")));
                return;
            };
            let Ok(amount) = amount.parse::<i32>() else {
                game.native_feedback(Text::literal("The amount must be a whole number."));
                return;
            };
            if !edit_card(game, pos, face, |card| {
                card.with_extract(card.channel().into(), amount)
            }) {
                game.native_feedback(Text::literal("LaserIO: no card on that side."));
                return;
            }
            let shown = laser_at(game, pos)
                .and_then(|entity| read_cards(&entity.nbt).get(face))
                .map_or(0, |card| card.amount());
            game.native_feedback(Text::literal(format!(
                "LaserIO: {} side moves {shown} item(s) every {OPERATION_TICK_DELAY} ticks",
                FACE_NAMES[face]
            )));
        }
        ["laserio", "face", x, y, z, side] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal("Usage: /laserio face <x> <y> <z> <side>"));
                return;
            };
            let Some(face) = parse_side(side) else {
                game.native_feedback(Text::literal(format!("Unknown side '{side}'.")));
                return;
            };
            if game.native_block(pos) != CONNECTOR {
                game.native_feedback(Text::literal("LaserIO: no laser connector there."));
                return;
            }
            game.native_set_block_property_with(
                pos,
                "facing",
                FACE_NAMES[face],
                flags::UPDATE_CLIENTS,
            );
            game.native_feedback(Text::literal(format!(
                "LaserIO: connector faces {}",
                FACE_NAMES[face]
            )));
        }
        _ => {}
    }
}
