//! Tests that the runtime's player matches the generated world's.

use greyhack_world::PlayerOptions;

#[test]
fn should_run_programs_as_the_player_of_the_generated_world() {
    assert_eq!(greybel_runtime::PLAYER_USER, PlayerOptions::default().user);
}
