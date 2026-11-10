// A placed laser node or connector declares a block entity, which saves the
// node's per-side cards (`native_block_entity`, API 15).
fn observe_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::laserio_state::on_block_entity(block, entity);
}
