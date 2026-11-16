use greyhack_behaviour::{creator_reward, player_reward};

#[test]
fn should_round_player_rewards_from_the_unrounded_float_expression() {
    assert_eq!(player_reward(1, 4, 10), 47);
}

#[test]
fn should_round_creator_rewards_from_the_unrounded_float_expression() {
    assert_eq!(creator_reward(0.01, 36, 1), 3);
}
