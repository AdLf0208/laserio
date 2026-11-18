//! LaserIO for Horizon: a laser node with one item card per side, and the
//! network of nodes and connectors that moves items between them.
//!
//! LaserIO (see `FINDINGS.md`) keeps cards on a laser node: an extract card pulls
//! items out of the inventory on its side, an insert card puts them into the
//! inventory on its side, and a laser connector relays the network onwards. This
//! slice keeps one card per side (upstream: nine), treats the contiguous run of
//! touching laser blocks as the network, and runs each extract card's 20-tick
//! operation at the end of the level's tick.
//!
//! The rules are what [`runtime`] decides (faces, cadence, amount, destination
//! order and the budget); [`laser`] is the game side:
//!
//! - [`laser`] holds the node's block entity part, the queue a node's tick fills,
//!   the per-network pass at the end of the level's tick, the card placement and
//!   the `/laserio` command handlers;
//! - this file is the one `impl Hooks`.
pub mod runtime;

mod laser;

use std::sync::{Arc, Mutex};

use mod_api::{
    self, Command, Flow, Hand, Hooks, HostSide, LevelTickPostEvent, RegisterCommandsEvent,
    Registrar, RegistrationContext, RightClickBlockEvent, RunModCommandEvent, ServerHost,
};

use crate::laser::{Node, Placement, Queue};

/// The mod's session state: the nodes whose block entity ticked this tick.
#[derive(Default)]
struct Mod {
    queue: Arc<Mutex<Queue>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the node's block entity type (the home of its per-side cards) and
    /// attach the one part that makes it tick. A refusal here can only mean the
    /// same part attached twice, which is a broken build rather than a world.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        let kind = registrar
            .block_entity_type("laser_node")
            .blocks(&["laser_node"])
            .build();
        let attached = registrar.block_entity_part(&kind, Node::new(Arc::clone(&self.queue)));
        debug_assert!(
            attached.is_ok(),
            "the laser node part attaches once: {attached:?}"
        );
    }

    /// Right-clicking a node with an item card places it on the clicked side
    /// (upstream's `LaserNode.useWithoutItem`); the main hand only, because the
    /// game fires this hook once per hand.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let click = event.click;
        if click.hand != Hand::Main {
            return Flow::Continue;
        }
        let Some(held) = click.stack else {
            return Flow::Continue;
        };
        match laser::place_card_from_item(
            host,
            click.pos,
            click.hit.face,
            held,
            click.player,
            click.hand,
        ) {
            Placement::NotMine => Flow::Continue,
            Placement::Placed { face } => {
                laser::tell(host, click.player, &laser::placement_line(face));
                Flow::Cancel
            }
            Placement::Taken { face } => {
                laser::tell(host, click.player, &laser::taken_face_line(face));
                Flow::Cancel
            }
        }
    }

    /// After the level's tick: run every queued node network's extract cards, one
    /// pass per network, in one transaction. A client's tick runs nothing.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        // The pass reaches containers through this level's `ITEMS` capability, and
        // `LevelRef::capability` answers only for the level the server ticks now.
        // Outside that window the queue is left alone and the next tick retries,
        // rather than dropping a node's work: the queue is bounded by the number of
        // nodes, so waiting costs nothing.
        if !event.level.in_place(host) {
            return;
        }
        let queued = {
            // Only this expression runs under the lock: one `remove` on the port's
            // own state, which cannot panic and so cannot poison it.
            self.queue
                .lock()
                .expect("the laser node queue lock is held for one drain")
                .take(event.level)
        };
        laser::run_pass(host, event.level, queued);
    }

    /// `/laserio info` is a readout; `card`/`remove`/`mode`/`rate`/`face` change a
    /// node's automation and are operator-only (level 2): this pin has no node
    /// screen, so they are a harness surface, not something every player may use
    /// on any node.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("laserio info <pos:block_pos>"));
        event
            .commands
            .push(Command::op("laserio card <pos:block_pos> <side:word>", 2));
        event
            .commands
            .push(Command::op("laserio remove <pos:block_pos> <side:word>", 2));
        event.commands.push(Command::op(
            "laserio mode <pos:block_pos> <side:word> <mode:word>",
            2,
        ));
        event.commands.push(Command::op(
            "laserio rate <pos:block_pos> <side:word> <amount:integer>",
            2,
        ));
        event
            .commands
            .push(Command::op("laserio face <pos:block_pos> <side:word>", 2));
    }

    /// Run the six usages [`Hooks::register_commands`] declares, after the game
    /// has parsed and permission-checked them.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        match literals.as_slice() {
            ["laserio", "info"] => laser::info(host, event.command),
            ["laserio", "card"] => laser::set_card_command(host, event.command, true),
            ["laserio", "remove"] => laser::set_card_command(host, event.command, false),
            ["laserio", "mode"] => laser::set_mode_command(host, event.command),
            ["laserio", "rate"] => laser::set_rate_command(host, event.command),
            ["laserio", "face"] => laser::set_face_command(host, event.command),
            _ => return Flow::Continue,
        }
        Flow::Cancel
    }
}
