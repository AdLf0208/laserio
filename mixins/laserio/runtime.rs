// Pure LaserIO card and network logic (the standalone crate lib, injected as
// `crate::game::laserio`). No game access lives here.
//
// Upstream a laser node holds cards per side; an EXTRACT card pulls a stack's
// `extractAmt` from the inventory on its side every `getExtractSpeed` ticks and
// hands it to an INSERT card in the same channel, which inserts it into the
// inventory on its side. With no upgrades those defaults are one item every 20
// ticks, channel 0, and nothing is extracted when no destination accepts it.

/// One of a node's six sides, in vanilla's `Direction.values()` order.
pub type Face = usize;

/// A block position, as the glue reads it from the level.
pub type Pos = [i32; 3];

pub const FACE_COUNT: usize = 6;

pub const DOWN: Face = 0;
pub const UP: Face = 1;
pub const NORTH: Face = 2;
pub const SOUTH: Face = 3;
pub const WEST: Face = 4;
pub const EAST: Face = 5;

/// `Direction.values()`'s names, which are also the connector's `facing` values.
pub const FACE_NAMES: [&str; FACE_COUNT] = ["down", "up", "north", "south", "west", "east"];

/// Upstream `BaseCard.getExtractSpeed` with no card overclocker: the ticks
/// between two operations of one card.
pub const OPERATION_TICK_DELAY: u64 = 20;

/// Upstream `CardItem.getItemExtractAmt` with no setting: the items one
/// operation moves.
pub const DEFAULT_AMOUNT: u32 = 1;

/// The most one operation moves: a full stack, which is what our `/laserio rate`
/// accepts at the top end (`Card` clamps to it).
pub const MAX_AMOUNT: u32 = 64;

/// Upstream's card channel is a byte but the settings screen offers 0–15.
pub const MAX_CHANNEL: u8 = 15;

/// The face's unit vector, as `Direction.normal()` gives it.
pub const fn face_normal(face: Face) -> [i32; 3] {
    match face {
        DOWN => [0, -1, 0],
        UP => [0, 1, 0],
        NORTH => [0, 0, -1],
        SOUTH => [0, 0, 1],
        WEST => [-1, 0, 0],
        EAST => [1, 0, 0],
        _ => [0, 0, 0],
    }
}

/// The position one step from `pos` through `face`.
pub const fn step(pos: Pos, face: Face) -> Pos {
    let [dx, dy, dz] = face_normal(face);
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// The side across a block from `face`.
pub const fn opposite(face: Face) -> Face {
    match face {
        DOWN => UP,
        UP => DOWN,
        NORTH => SOUTH,
        SOUTH => NORTH,
        WEST => EAST,
        EAST => WEST,
        _ => face,
    }
}

/// The face a state property or command names (`down`..`east`), if it names one.
pub fn face_named(name: &str) -> Option<Face> {
    FACE_NAMES.iter().position(|known| *known == name)
}

/// What a card does with the inventory on its side: upstream's
/// `BaseCard.TransferMode`, reduced to the two modes this slice operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransferMode {
    /// Accepts items from the network and inserts them into its side's
    /// inventory (upstream's default when a card is crafted).
    Insert,
    /// Pulls items from its side's inventory into the network.
    Extract,
}

impl TransferMode {
    /// The mode a command names, if it names one.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "insert" => Some(Self::Insert),
            "extract" => Some(Self::Extract),
            _ => None,
        }
    }

    /// The mode's command/readout name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Extract => "extract",
        }
    }

    const fn bit(self) -> i32 {
        match self {
            Self::Insert => 0,
            Self::Extract => 1,
        }
    }

    const fn from_bit(bit: i32) -> Self {
        if bit & 1 == 1 {
            Self::Extract
        } else {
            Self::Insert
        }
    }
}

/// One card's settings. Upstream keeps these in the card item's components;
/// this slice keeps them in the node's block entity, with the same meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    mode: TransferMode,
    channel: u8,
    amount: u32,
}

impl Card {
    /// A freshly crafted item card: an inserter, channel 0, one item.
    pub const fn new() -> Self {
        Card {
            mode: TransferMode::Insert,
            channel: 0,
            amount: DEFAULT_AMOUNT,
        }
    }

    /// The same card in another mode, keeping its channel and amount.
    pub const fn with_mode(self, mode: TransferMode) -> Self {
        Card { mode, ..self }
    }

    /// The same card with a new extract amount and channel, clamped to the
    /// values the settings offer (`Card` never carries an invalid setting).
    pub fn with_extract(self, channel: i32, amount: i32) -> Self {
        Card {
            channel: u8::try_from(channel.clamp(0, i32::from(MAX_CHANNEL))).unwrap_or(MAX_CHANNEL),
            amount: u32::try_from(amount.clamp(1, i32::try_from(MAX_AMOUNT).unwrap_or(i32::MAX)))
                .unwrap_or(DEFAULT_AMOUNT),
            ..self
        }
    }

    pub const fn mode(self) -> TransferMode {
        self.mode
    }

    pub const fn channel(self) -> u8 {
        self.channel
    }

    pub const fn amount(self) -> u32 {
        self.amount
    }

    /// The block-entity integer this card saves as: mode bit, channel, amount.
    pub const fn saved(self) -> i32 {
        self.mode.bit() | (self.channel as i32) << 1 | (self.amount as i32) << 5
    }

    /// Restore a card [`Card::saved`] wrote; anything out of range is clamped,
    /// so a damaged save is a valid card rather than a panic.
    pub fn from_saved(saved: i32) -> Self {
        Card {
            mode: TransferMode::from_bit(saved),
            channel: u8::try_from((saved >> 1) & 0xF).unwrap_or(0),
            amount: u32::try_from(saved >> 5)
                .unwrap_or(MAX_AMOUNT)
                .clamp(DEFAULT_AMOUNT, MAX_AMOUNT),
        }
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}

/// The cards one node holds: upstream has nine slots per side; this slice keeps
/// one card per side, which is the model the readout and the transfer use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cards {
    sides: [Option<Card>; FACE_COUNT],
}

impl Cards {
    /// A node with no cards.
    pub const fn new() -> Self {
        Cards {
            sides: [None; FACE_COUNT],
        }
    }

    /// The card on `face`, if the face has one.
    pub fn get(&self, face: Face) -> Option<Card> {
        self.sides.get(face).copied().flatten()
    }

    /// The cards with `face` set to `card` (or cleared); a face past the six
    /// sides is ignored.
    pub fn with(self, face: Face, card: Option<Card>) -> Self {
        let mut sides = self.sides;
        if let Some(slot) = sides.get_mut(face) {
            *slot = card;
        }
        Cards { sides }
    }

    /// The cards this node holds, in face order.
    pub fn cards(&self) -> impl Iterator<Item = (Face, Card)> + '_ {
        self.sides
            .iter()
            .enumerate()
            .filter_map(|(face, card)| card.map(|card| (face, card)))
    }

    /// How many sides carry a card.
    pub fn count(&self) -> usize {
        self.sides.iter().flatten().count()
    }

    /// Whether the node has no cards at all.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

/// An inventory a network can move into: the block that reaches it and the
/// side through which it reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint {
    pub laser: Pos,
    pub face: Face,
}

/// A card's operation phase, from its block and side: it spreads a level's
/// cards over the 20-tick cadence instead of moving them all on one tick
/// (upstream ticks each card on its own `remainingSleep` counter, which a
/// block-entity write inside the tick would lose at this pin).
pub fn phase_of(pos: Pos, face: Face) -> u64 {
    (i64::from(pos[0]) + i64::from(pos[1]) + i64::from(pos[2]) + face as i64)
        .rem_euclid(OPERATION_TICK_DELAY as i64) as u64
}

/// Whether this tick is the card's operation tick.
pub fn is_operation_tick(game_time: u64, phase: u64) -> bool {
    game_time
        .wrapping_add(phase)
        .is_multiple_of(OPERATION_TICK_DELAY)
}

/// The order one operation tries the network's destinations in: block position
/// then side, so the same network always gives the same order and the source
/// endpoint is never a destination of itself.
pub fn endpoint_order(endpoints: &[Endpoint], source: Endpoint) -> Vec<usize> {
    let mut order: Vec<usize> = (0..endpoints.len())
        .filter(|&index| endpoints[index] != source)
        .collect();
    order.sort_by_key(|&index| endpoints[index]);
    order
}

/// Run one operation's budget down the destination `order`: each destination
/// takes what it can, and the rest goes to the next. Returns how much moved,
/// never more than `budget`. An empty `order` moves nothing and asks no one,
/// which is why a card with no destination does not extract at all.
pub fn spend_budget(
    budget: u32,
    order: &[usize],
    mut insert: impl FnMut(usize, u32) -> u32,
) -> u32 {
    let mut spent = 0;
    for &index in order {
        let remaining = budget - spent;
        if remaining == 0 {
            break;
        }
        spent += insert(index, remaining).min(remaining);
    }
    spent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_follow_vanillas_direction_order() {
        assert_eq!(FACE_NAMES, ["down", "up", "north", "south", "west", "east"]);
        for (index, name) in FACE_NAMES.iter().enumerate() {
            assert_eq!(face_named(name), Some(index), "{name}");
        }
        assert_eq!(face_named("sideways"), None);
        assert_eq!(face_normal(NORTH), [0, 0, -1]);
        assert_eq!(face_normal(EAST), [1, 0, 0]);
        assert_eq!(face_normal(SOUTH), [0, 0, 1]);
        assert_eq!(opposite(WEST), EAST);
        assert_eq!(step([4, 5, 6], UP), [4, 6, 6]);
        assert_eq!(step([4, 5, 6], WEST), [3, 5, 6]);
    }

    #[test]
    fn the_rate_is_one_item_every_twenty_ticks() {
        assert_eq!(DEFAULT_AMOUNT, 1, "upstream `CardItem.getItemExtractAmt`");
        assert_eq!(
            OPERATION_TICK_DELAY, 20,
            "upstream `BaseCard.getExtractSpeed`"
        );
        assert_eq!(MAX_AMOUNT, 64, "one stack per operation at most");
        assert_eq!(MAX_CHANNEL, 15, "upstream's channel is a 4-bit choice");
    }

    #[test]
    fn an_operation_runs_every_twenty_ticks_offset_by_position_and_side() {
        // The node at (1, 100, 0), west side: 1 + 100 + 0 + 4 == 105, 105 % 20 == 5.
        let phase = phase_of([1, 100, 0], WEST);
        assert_eq!(phase, 5);
        assert!(is_operation_tick(15, phase));
        assert!(!is_operation_tick(16, phase));
        assert!(is_operation_tick(35, phase));
        assert!(!is_operation_tick(0, phase));
        // Two sides of the same node do not fire together.
        assert_ne!(phase_of([1, 100, 0], EAST), phase);
    }

    #[test]
    fn a_fresh_card_is_an_inserter_on_channel_zero_with_one_item() {
        let card = Card::new();
        assert_eq!(card.mode(), TransferMode::Insert);
        assert_eq!(card.channel(), 0);
        assert_eq!(card.amount(), 1);
        assert_eq!(Card::from_saved(card.saved()), card);
    }

    #[test]
    fn a_card_clamps_its_channel_and_amount() {
        let card = Card::new().with_mode(TransferMode::Extract);
        assert_eq!(card.with_extract(99, 0).channel(), MAX_CHANNEL);
        assert_eq!(card.with_extract(99, 0).amount(), 1);
        assert_eq!(card.with_extract(0, 200).amount(), MAX_AMOUNT);
        assert_eq!(card.with_extract(0, -5).amount(), DEFAULT_AMOUNT);
        assert_eq!(card.with_extract(3, 7).amount(), 7);
        assert_eq!(card.with_extract(3, 7).channel(), 3);
        assert_eq!(
            card.with_mode(TransferMode::Extract).mode(),
            TransferMode::Extract
        );
    }

    #[test]
    fn mode_names_round_trip() {
        assert_eq!(TransferMode::named("insert"), Some(TransferMode::Insert));
        assert_eq!(TransferMode::named("extract"), Some(TransferMode::Extract));
        assert_eq!(TransferMode::named("stock"), None);
        assert_eq!(TransferMode::Insert.name(), "insert");
        assert_eq!(TransferMode::Extract.name(), "extract");
    }

    #[test]
    fn a_node_keeps_one_card_per_side() {
        let mut cards = Cards::new();
        assert!(cards.is_empty());
        assert_eq!(cards.get(WEST), None);
        assert_eq!(cards.count(), 0);
        cards = cards.with(WEST, Some(Card::new()));
        assert_eq!(cards.get(WEST), Some(Card::new()));
        assert_eq!(cards.get(EAST), None);
        assert_eq!(cards.count(), 1);
        assert_eq!(
            cards.cards().collect::<Vec<(Face, Card)>>(),
            vec![(WEST, Card::new())]
        );
        cards = cards.with(WEST, None);
        assert!(cards.is_empty());
        // A face past the six sides is ignored rather than panicking.
        assert_eq!(cards.with(FACE_COUNT, Some(Card::new())), cards);
    }

    #[test]
    fn destinations_are_deterministic_and_never_the_source() {
        let endpoints = [
            Endpoint {
                laser: [9, 0, 0],
                face: UP,
            },
            Endpoint {
                laser: [2, 0, 0],
                face: WEST,
            },
            Endpoint {
                laser: [2, 0, 0],
                face: DOWN,
            },
            Endpoint {
                laser: [1, 0, 0],
                face: WEST,
            },
        ];
        let source = endpoints[3];
        assert_eq!(endpoint_order(&endpoints, source), vec![2, 1, 0]);
        // The same network read in another order gives the same order.
        let reversed = [endpoints[0], endpoints[1], endpoints[2], endpoints[3]];
        assert_eq!(endpoint_order(&reversed, source), vec![2, 1, 0]);
        // A lone endpoint is not its own destination.
        assert!(endpoint_order(&[source], source).is_empty());
    }

    #[test]
    fn the_budget_spills_to_the_next_destination() {
        let mut calls: Vec<(usize, u32)> = Vec::new();
        let spent = spend_budget(DEFAULT_AMOUNT, &[0, 1], |index, remaining| {
            calls.push((index, remaining));
            remaining.min(if index == 0 { 0 } else { 1 })
        });
        assert_eq!(spent, 1);
        assert_eq!(calls, vec![(0, 1), (1, 1)]);
    }

    #[test]
    fn a_network_without_destinations_moves_nothing() {
        assert_eq!(
            spend_budget(DEFAULT_AMOUNT, &[], |_, _| unreachable!("no destination")),
            0
        );
        let mut calls = 0;
        let spent = spend_budget(4, &[0], |_, _| {
            calls += 1;
            0
        });
        assert_eq!(spent, 0);
        assert_eq!(calls, 1, "the destination was asked and refused");
    }
}
