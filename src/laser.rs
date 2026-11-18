//! Game glue for LaserIO's laser node, its connector and the item card: the
//! per-side cards, the network a node and its connectors form, the 20-tick
//! operation that moves items, and the `/laserio` readout.
//!
//! Upstream a laser node keeps nine card slots per side and discovers its
//! network through wrench links; an extract card hands a stack's worth of items
//! to an insert card in the same channel. This slice keeps **one card per side**
//! in the node's block entity (encoded as [`Card::saved`]), treats the
//! contiguous run of touching laser blocks as the network, and after the level's
//! tick runs each extract card's operation into the inventories the network
//! reaches.
//!
//! A node's block entity ticks, and its tick only **queues** the node; the
//! network's moves run once at `level_tick_post`, in one transaction, so a
//! network of many lasers moves items once per tick and not once per block. The
//! rules — which face, which cadence, how a budget spills — are [`crate::runtime`]'s;
//! this file reads the world, calls those rules and writes the result back.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use mod_api::{
    BlockEntityPart, BlockEntityTickEvent, BlockEntityTicksEvent, BlockKey, BlockPart, Capability,
    CommandArgumentValue, CommandInvocation, Compound, Direction, Hand, Host, HostTransaction,
    ITEMS, ItemPlace, ItemResource, LevelRef, ResourceHandler, ServerHost, Tag, Text, Transaction,
};

use crate::runtime::{
    Card, Cards, Endpoint, FACE_COUNT, FACE_NAMES, Face, OPERATION_TICK_DELAY, Pos, TransferMode,
    endpoint_order, face_named, face_normal, is_operation_tick, opposite, phase_of, spend_budget,
    step,
};

/// The laser node block (and its item): the block that carries the cards.
pub(crate) const NODE: &str = "laserio:laser_node";
/// The connector block (and its item).
pub(crate) const CONNECTOR: &str = "laserio:laser_connector";
/// The item card.
pub(crate) const CARD: &str = "laserio:card_item";

/// The settings key one side saves its card under (the shape 0.1.0 wrote).
fn card_key(face: Face) -> String {
    format!("card_{}", FACE_NAMES[face])
}

/// A safety cap on one network's laser count (upstream has no limit; a runaway
/// build must not stall the level tick).
const MAX_NETWORK: usize = 1024;

/// Nodes whose block entity ticked this tick, per level, drained at the end of
/// that level's tick.
#[derive(Default)]
pub(crate) struct Queue {
    pending: std::collections::HashMap<LevelRef, BTreeSet<Pos>>,
}

impl Queue {
    /// Queue `pos` for the level the node's tick ran in.
    fn push(&mut self, level: LevelRef, pos: Pos) {
        self.pending.entry(level).or_default().insert(pos);
    }

    /// Everything queued for `level`, in position order (empty afterwards).
    pub(crate) fn take(&mut self, level: LevelRef) -> BTreeSet<Pos> {
        self.pending.remove(&level).unwrap_or_default()
    }
}

/// The part this mod attaches to the laser node's block entity type: it makes a
/// placed node tick, and its tick only queues the node for the level tick's end.
pub(crate) struct Node {
    queue: Arc<Mutex<Queue>>,
}

impl Node {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> Node {
        Node { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for Node {}

#[mod_api::horizon_part]
impl BlockEntityPart for Node {
    /// A node ticks on the server. The part is attached to the node's type only,
    /// so every block it is asked about is a laser node.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One node tick: queue it for the level tick's end.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let Some(level) = level_in_place(host) else {
            return;
        };
        // Only this expression runs under the lock: one insert on the port's own
        // state, which cannot panic and so cannot poison it.
        self.queue
            .lock()
            .expect("the laser node queue lock is held for one insert")
            .push(level, event.pos);
    }
}

/// The level the server is ticking now: the one a block-entity tick runs in.
fn level_in_place(host: &dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|&level| level.in_place(host))
}

/// The level a click or a command acts in: the acting player's own.
fn acting_level(host: &dyn ServerHost) -> Option<LevelRef> {
    let player = host.players().acting_player()?;
    player.level(host)
}

/// The two block keys this mod owns, resolved once per pass (`BlockKey` is the
/// engine's own identity: comparing it costs no string).
#[derive(Clone, Copy)]
struct Lasers {
    node: BlockKey,
    connector: BlockKey,
}

impl Lasers {
    fn resolve(host: &dyn Host) -> Option<Lasers> {
        Some(Lasers {
            node: host.block_key(NODE)?,
            connector: host.block_key(CONNECTOR)?,
        })
    }
}

/// Whether the block at `pos` is `key`.
fn is_block(host: &dyn Host, key: BlockKey, level: LevelRef, pos: Pos) -> bool {
    level.block(host, pos) == Some(key)
}

/// Whether the block at `pos` is a node or a connector.
fn is_laser(host: &dyn Host, lasers: Lasers, level: LevelRef, pos: Pos) -> bool {
    is_block(host, lasers.node, level, pos) || is_block(host, lasers.connector, level, pos)
}

/// A node's cards; a fresh node has none.
fn cards_at(host: &dyn Host, level: LevelRef, pos: Pos) -> Cards {
    let Some(entity) = host.block_entity(level, pos) else {
        return Cards::new();
    };
    let Some(data) = entity.data(host) else {
        return Cards::new();
    };
    let mut cards = Cards::new();
    for face in 0..FACE_COUNT {
        let key = card_key(face);
        if let Some(Tag::Int(saved)) = data.get(&key.as_str().into()) {
            cards = cards.with(face, Some(Card::from_saved(*saved)));
        }
    }
    cards
}

/// Save a node's cards, as `/data merge block` does: the payload is replaced, so a
/// removed card's key is gone rather than left behind, and the write is part of
/// the caller's transaction.
fn set_cards(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, cards: Cards) -> bool {
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let mut data = Compound::new();
    for face in 0..FACE_COUNT {
        if let Some(card) = cards.get(face) {
            data.insert(card_key(face).as_str().into(), Tag::Int(card.saved()));
        }
    }
    // `Ok(false)` is "no change": this compound holds ints only, so the other
    // causes `replace_data` names (too deep, cannot load) cannot apply to it.
    entity.replace_data(host, data).is_ok()
}

/// The network a laser block belongs to: every node and connector reachable
/// through touching laser blocks (upstream walks wrench links instead).
fn network_of(host: &dyn Host, lasers: Lasers, level: LevelRef, start: Pos) -> Vec<Pos> {
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    let mut queue = vec![start];
    seen.insert(start);
    while let Some(pos) = queue.pop() {
        if seen.len() >= MAX_NETWORK {
            break;
        }
        for face in 0..FACE_COUNT {
            let beside = step(pos, face);
            if is_laser(host, lasers, level, beside) && seen.insert(beside) {
                queue.push(beside);
            }
        }
    }
    seen.into_iter().collect()
}

/// The side of the neighbouring block that touches a laser's `face`.
fn touching_side(face: Face) -> Direction {
    Direction::ALL[opposite(face)]
}

/// The container at `pos` reached through `face`, as the engine's side-filtered
/// item handler.
fn container(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
) -> Option<Capability<dyn ResourceHandler<ItemResource>>> {
    level.capability(host, &ITEMS, pos, Some(touching_side(face)))
}

/// Whether the block at `pos` reaches an item container through `face`.
fn has_container(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, face: Face) -> bool {
    container(host, level, pos, face).is_some_and(|handler| handler.size(host) > 0)
}

/// The side a connector points at, from its `facing` state.
fn connector_face(host: &dyn Host, level: LevelRef, pos: Pos) -> Option<Face> {
    level
        .block_property(host, pos, "facing")
        .and_then(|value| face_named(&value))
}

/// The inventories a network can insert into: a node's INSERT cards and a
/// connector's `facing` side, when that side reaches a container.
fn endpoints(
    host: &mut dyn ServerHost,
    lasers: Lasers,
    level: LevelRef,
    network: &[Pos],
) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    for &laser in network {
        let faces: Vec<Face> = if is_block(host, lasers.connector, level, laser) {
            connector_face(host, level, laser).into_iter().collect()
        } else {
            cards_at(host, level, laser)
                .cards()
                .filter(|(_, card)| card.mode() == TransferMode::Insert)
                .map(|(face, _)| face)
                .collect()
        };
        for face in faces {
            if has_container(host, level, step(laser, face), opposite(face)) {
                endpoints.push(Endpoint { laser, face });
            }
        }
    }
    endpoints
}

/// The first slot the source side offers that holds something: one item type per
/// operation, as upstream's `sendItems` takes the first valid stack. An empty
/// earlier slot must not hide the items behind it.
fn first_item(
    host: &mut dyn ServerHost,
    source: &Capability<dyn ResourceHandler<ItemResource>>,
) -> Option<(usize, ItemResource)> {
    let size = source.size(host);
    for slot in 0..size {
        if let Some(resource) = source.resource(host, slot) {
            return Some((slot, resource));
        }
    }
    None
}

/// What one pass needs to know that is the same for every card of a network.
#[derive(Clone, Copy)]
struct Pass<'a> {
    lasers: Lasers,
    level: LevelRef,
    network: &'a [Pos],
}

/// Run one extract card of `pass.network` for this tick: move up to its amount
/// into the network's inventories, and nothing at all when there is no
/// destination (upstream extracts only what an inserter accepts).
///
/// What a destination refuses is put back into the source in the same
/// transaction, so a full target cannot lose an item.
fn run_card(
    host: &mut dyn ServerHost,
    tx: &Transaction,
    pass: Pass<'_>,
    laser: Pos,
    face: Face,
    card: Card,
) {
    let Pass { lasers, level, .. } = pass;
    let from = step(laser, face);
    let from_face = opposite(face);
    let Some(source) = container(host, level, from, from_face) else {
        return;
    };
    let Some((slot, resource)) = first_item(host, &source) else {
        return;
    };
    // The readout and the move agree on the destination set: the same endpoints,
    // in the same order, the source never among them.
    let all = endpoints(host, lasers, level, pass.network);
    let order = endpoint_order(&all, Endpoint { laser, face });
    if order.is_empty() {
        return;
    }
    spend_budget(card.amount(), &order, |index, remaining| {
        let destination = all[index];
        let to_pos = step(destination.laser, destination.face);
        let Some(target) = container(host, level, to_pos, opposite(destination.face)) else {
            return 0;
        };
        let took = source.extract(host, tx, slot, &resource, remaining);
        if took == 0 {
            return 0;
        }
        let put = target.insert_any(host, tx, &resource, took);
        if put < took {
            // Give the refused part back: the slot it came from has room for it
            // again, and the whole move is inside one transaction.
            let _ = source.insert_any(host, tx, &resource, took - put);
        }
        put
    });
}

/// Run every queued node's network at the end of the level's tick: each extract
/// card whose 20-tick operation falls on this tick moves its items, once per
/// network, in one transaction.
pub(crate) fn run_pass(host: &mut dyn ServerHost, level: LevelRef, queued: BTreeSet<Pos>) {
    if queued.is_empty() {
        return;
    }
    let Some(lasers) = Lasers::resolve(host) else {
        return;
    };
    // The cadence is the level's own game time, as 0.1.0 read it.
    let Some(game_time) = level.game_time(host) else {
        return;
    };
    let game_time = game_time.max(0) as u64;
    let mut done: BTreeSet<Pos> = BTreeSet::new();
    for start in queued {
        if done.contains(&start) || !is_block(host, lasers.node, level, start) {
            continue;
        }
        let network = network_of(host, lasers, level, start);
        done.extend(network.iter().copied());
        let pass = Pass {
            lasers,
            level,
            network: &network,
        };
        // One transaction per network per tick: the cards of one network move
        // together, and a refused move never leaves an item in flight.
        let _ = host.transaction(|host, tx| {
            for &laser in &network {
                if !is_block(host, lasers.node, level, laser) {
                    continue;
                }
                for (face, card) in cards_at(host, level, laser).cards() {
                    if card.mode() != TransferMode::Extract
                        || !is_operation_tick(game_time, phase_of(laser, face))
                    {
                        continue;
                    }
                    run_card(host, tx, pass, laser, face, card);
                }
            }
            Ok::<(), ()>(())
        });
    }
}

/// The face a clicked normal names, if it names one.
fn face_of_normal(normal: [i32; 3]) -> Option<Face> {
    (0..FACE_COUNT).find(|&face| face_normal(face) == normal)
}

/// What a right click with an item card did.
pub(crate) enum Placement {
    /// The click was not this mod's business.
    NotMine,
    /// The card went onto `face`.
    Placed { face: Face },
    /// The node's `face` already holds a card; the (unported) screen would open.
    Taken { face: Face },
}

/// Put a fresh card on one side of the node at `pos`. Answers `None` when the
/// block is not a node or the write was refused.
fn put_card(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, face: Face) -> Option<bool> {
    if !is_block(host, Lasers::resolve(host)?.node, level, pos) {
        return None;
    }
    let current = cards_at(host, level, pos);
    if current.get(face).is_some() {
        return Some(false);
    }
    set_cards(host, level, pos, current.with(face, Some(Card::new()))).then_some(true)
}

/// Right-clicking a node with an item card places it on the clicked side, as
/// upstream's `LaserNode.useWithoutItem` does; the item is consumed outside
/// creative. A side that already carries a card is left for the (unported) menu.
pub(crate) fn place_card_from_item(
    host: &mut dyn ServerHost,
    pos: Pos,
    normal: [i32; 3],
    held: &mod_api::ItemStack,
    player: u128,
    hand: Hand,
) -> Placement {
    let Some(face) = face_of_normal(normal) else {
        return Placement::NotMine;
    };
    if held.item().id(host) != CARD {
        return Placement::NotMine;
    }
    let Some(level) = acting_level(host) else {
        return Placement::NotMine;
    };
    match put_card(host, level, pos, face) {
        None => Placement::NotMine,
        Some(false) => Placement::Taken { face },
        Some(true) => {
            // `ItemPlace::extract` on a creative player's hand gives without
            // losing any, which is the rule this replaces (`game_mode != 1`).
            let resource = ItemResource::of(held);
            let place = ItemPlace::PlayerHand { player, hand };
            let _ = host.transaction(|host, tx| Ok::<_, ()>(place.extract(host, tx, &resource, 1)));
            Placement::Placed { face }
        }
    }
}

/// Replace the card on a side with an edited copy; false when the block is not a
/// node, the side has no card, or the write was refused.
fn edit_card(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
    edit: impl FnOnce(Card) -> Card,
) -> bool {
    let Some(lasers) = Lasers::resolve(host) else {
        return false;
    };
    if !is_block(host, lasers.node, level, pos) {
        return false;
    }
    let current = cards_at(host, level, pos);
    let Some(card) = current.get(face) else {
        return false;
    };
    set_cards(host, level, pos, current.with(face, Some(edit(card))))
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

/// `/laserio info <pos>`: the cards, the network members, the targets and each
/// card's amount and cadence.
pub(crate) fn info(host: &mut dyn ServerHost, command: &CommandInvocation) {
    let Some(pos) = command_pos(command) else {
        feedback(host, "Usage: /laserio info <x> <y> <z>");
        return;
    };
    let Some(level) = acting_level(host) else {
        feedback(host, "LaserIO: no level to read.");
        return;
    };
    let Some(lasers) = Lasers::resolve(host) else {
        return;
    };
    let connector = is_block(host, lasers.connector, level, pos);
    if !connector && !is_block(host, lasers.node, level, pos) {
        feedback(host, "LaserIO: no laser node or connector there.");
        return;
    }
    let cards = if connector {
        Cards::new()
    } else {
        cards_at(host, level, pos)
    };
    if connector {
        let facing = connector_face(host, level, pos)
            .map(|face| FACE_NAMES[face])
            .unwrap_or("none");
        feedback(
            host,
            &format!(
                "Laser Connector at {},{},{}: facing {facing}",
                pos[0], pos[1], pos[2]
            ),
        );
    } else {
        feedback(
            host,
            &format!(
                "Laser Node at {},{},{}: cards {}",
                pos[0],
                pos[1],
                pos[2],
                card_list(cards)
            ),
        );
    }
    let network = network_of(host, lasers, level, pos);
    let members: Vec<String> = network
        .iter()
        .map(|member| format!("{}, {}, {}", member[0], member[1], member[2]))
        .collect();
    feedback(
        host,
        &format!(
            "Laser Node at {},{},{}: network {}",
            pos[0],
            pos[1],
            pos[2],
            if members.is_empty() {
                "none".to_string()
            } else {
                members.join(" ")
            }
        ),
    );
    let targets = target_list(&endpoints(host, lasers, level, &network));
    feedback(
        host,
        &format!(
            "Laser Node at {},{},{}: targets {targets}",
            pos[0], pos[1], pos[2]
        ),
    );
    for (face, card) in cards.cards() {
        feedback(
            host,
            &format!(
                "Laser Node at {},{},{}: {} {} channel {}, {} item(s) every {} ticks",
                pos[0],
                pos[1],
                pos[2],
                FACE_NAMES[face],
                card.mode().name(),
                card.channel(),
                card.amount(),
                OPERATION_TICK_DELAY
            ),
        );
    }
}

/// `/laserio card|remove <pos> <side>`: the harness path for a card on an
/// occluded side (`ctl interact` only reaches a visible face).
pub(crate) fn set_card_command(
    host: &mut dyn ServerHost,
    command: &CommandInvocation,
    place: bool,
) {
    let usage = if place {
        "Usage: /laserio card <x> <y> <z> <side>"
    } else {
        "Usage: /laserio remove <x> <y> <z> <side>"
    };
    let Some(level) = acting_level(host) else {
        feedback(host, "LaserIO: no level to write.");
        return;
    };
    let Some((pos, face)) = command_target(host, command, usage) else {
        return;
    };
    if !is_node(host, level, pos) {
        feedback(host, "LaserIO: no laser node there.");
        return;
    }
    let current = cards_at(host, level, pos);
    if place && current.get(face).is_some() {
        feedback(
            host,
            &format!("LaserIO: the {} side already has a card.", FACE_NAMES[face]),
        );
        return;
    }
    if !place && current.get(face).is_none() {
        feedback(
            host,
            &format!("LaserIO: the {} side has no card.", FACE_NAMES[face]),
        );
        return;
    }
    let cards = current.with(face, place.then(Card::new));
    if !set_cards(host, level, pos, cards) {
        feedback(host, "LaserIO: the node refused that card.");
        return;
    }
    feedback(
        host,
        &format!(
            "LaserIO: card on the {} side {}",
            FACE_NAMES[face],
            if place { "placed" } else { "removed" }
        ),
    );
}

/// Whether `pos` holds a laser node.
fn is_node(host: &dyn Host, level: LevelRef, pos: Pos) -> bool {
    Lasers::resolve(host).is_some_and(|lasers| is_block(host, lasers.node, level, pos))
}

/// `/laserio mode <pos> <side> <mode>`: switch one card between insert and extract.
pub(crate) fn set_mode_command(host: &mut dyn ServerHost, command: &CommandInvocation) {
    let Some(level) = acting_level(host) else {
        feedback(host, "LaserIO: no level to write.");
        return;
    };
    let Some((pos, face)) = command_target(
        host,
        command,
        "Usage: /laserio mode <x> <y> <z> <side> <mode>",
    ) else {
        return;
    };
    let Some(name) = command_word(command, "mode") else {
        feedback(host, "Usage: /laserio mode <x> <y> <z> <side> <mode>");
        return;
    };
    let Some(mode) = TransferMode::named(&name.to_lowercase()) else {
        feedback(host, "Unknown mode: insert or extract.");
        return;
    };
    if !edit_card(host, level, pos, face, |card| card.with_mode(mode)) {
        feedback(host, "LaserIO: no card on that side.");
        return;
    }
    feedback(
        host,
        &format!("LaserIO: {} side is now {}", FACE_NAMES[face], mode.name()),
    );
}

/// `/laserio rate <pos> <side> <amount>`: how many items one operation moves.
pub(crate) fn set_rate_command(host: &mut dyn ServerHost, command: &CommandInvocation) {
    let Some(level) = acting_level(host) else {
        feedback(host, "LaserIO: no level to write.");
        return;
    };
    let Some((pos, face)) = command_target(
        host,
        command,
        "Usage: /laserio rate <x> <y> <z> <side> <amount>",
    ) else {
        return;
    };
    let Some(amount) = command_integer(command, "amount") else {
        feedback(host, "The amount must be a whole number.");
        return;
    };
    if !edit_card(host, level, pos, face, |card| {
        card.with_extract(card.channel().into(), amount)
    }) {
        feedback(host, "LaserIO: no card on that side.");
        return;
    }
    let shown = cards_at(host, level, pos)
        .get(face)
        .map_or(0, |card| card.amount());
    feedback(
        host,
        &format!(
            "LaserIO: {} side moves {shown} item(s) every {OPERATION_TICK_DELAY} ticks",
            FACE_NAMES[face]
        ),
    );
}

/// `/laserio face <pos> <side>`: point a connector at one side.
pub(crate) fn set_face_command(host: &mut dyn ServerHost, command: &CommandInvocation) {
    let Some(level) = acting_level(host) else {
        feedback(host, "LaserIO: no level to write.");
        return;
    };
    let Some((pos, face)) =
        command_target(host, command, "Usage: /laserio face <x> <y> <z> <side>")
    else {
        return;
    };
    let Some(lasers) = Lasers::resolve(host) else {
        return;
    };
    if !is_block(host, lasers.connector, level, pos) {
        feedback(host, "LaserIO: no laser connector there.");
        return;
    }
    if !host.set_block_property(pos, "facing", FACE_NAMES[face]) {
        feedback(host, "LaserIO: the connector refused that side.");
        return;
    }
    feedback(
        host,
        &format!("LaserIO: connector faces {}", FACE_NAMES[face]),
    );
}

/// The `pos` argument of a command as a block position.
fn command_pos(command: &CommandInvocation) -> Option<Pos> {
    match command.argument("pos")? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

/// A `word` argument of a command.
fn command_word<'a>(command: &'a CommandInvocation, name: &str) -> Option<&'a str> {
    match command.argument(name)? {
        CommandArgumentValue::String(word) => Some(word),
        _ => None,
    }
}

/// The `side` argument of a command, as a face, or the line to answer a bad one
/// with: the six names the model has, as 0.1.0 did.
fn command_face(command: &CommandInvocation) -> Result<Face, String> {
    let Some(word) = command_word(command, "side") else {
        return Err("Usage: the side is down, up, north, south, west or east.".to_string());
    };
    face_named(&word.to_lowercase())
        .ok_or_else(|| format!("Unknown side '{word}': down, up, north, south, west or east."))
}

/// The `pos` and `side` arguments of a command, or the line to answer a bad one
/// with (the usage when the position is missing, the side's names when the side
/// is wrong).
fn command_target(
    host: &mut dyn ServerHost,
    command: &CommandInvocation,
    usage: &str,
) -> Option<(Pos, Face)> {
    let Some(pos) = command_pos(command) else {
        feedback(host, usage);
        return None;
    };
    match command_face(command) {
        Ok(face) => Some((pos, face)),
        Err(line) => {
            feedback(host, &line);
            None
        }
    }
}

/// An `integer` argument of a command.
fn command_integer(command: &CommandInvocation, name: &str) -> Option<i32> {
    match command.argument(name)? {
        CommandArgumentValue::Integer(value) => Some(*value),
        _ => None,
    }
}

/// Answer the command's source (its silence, track-output and swapped-in player
/// rules apply, as for any command).
fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// A reply to the player who right-clicked, outside the command path.
pub(crate) fn tell(host: &mut dyn ServerHost, player: u128, line: &str) {
    let Some(player) = host.player(player) else {
        return;
    };
    let _ = player.tell(host, Text::literal(line));
}

/// What a placed or refused card should say.
pub(crate) fn placement_line(face: Face) -> String {
    format!("LaserIO: item card placed on the {} side", FACE_NAMES[face])
}

/// The line a side that already holds a card answers with.
pub(crate) fn taken_face_line(face: Face) -> String {
    format!(
        "LaserIO: this side already has a card ({})",
        FACE_NAMES[face]
    )
}
