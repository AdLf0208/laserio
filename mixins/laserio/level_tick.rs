// After the level's tick: run every extract card whose 20-tick operation falls
// on this tick (`native_level_tick_post`, API 23).
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::laserio_state::on_level_tick_post(game, dimension);
}
