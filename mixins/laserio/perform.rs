// A typed `/laserio` command reaches the native command hook.
fn observe_perform(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::laserio_state::on_perform_command(game, player, command, ran);
}
