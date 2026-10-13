use greyhack_behaviour::CoinLimit;

/// A mining reward is its integer part, from one coin up to the most coins that can exist
/// (`CoinIntrinsics.set_reward`).
#[test]
fn should_accept_a_reward_of_one_coin_up_to_the_coin_maximum() {
    assert_eq!(CoinLimit::check_reward(1.9), Ok(1));
    assert_eq!(CoinLimit::check_reward(100_000_000.0), Ok(100_000_000));
    assert_eq!(
        CoinLimit::check_reward(100_000_001.0),
        Err(CoinLimit::RewardTooHigh)
    );
    assert_eq!(CoinLimit::check_reward(0.9), Err(CoinLimit::RewardTooLow));
    assert_eq!(
        CoinLimit::check_reward(f64::NAN),
        Err(CoinLimit::RewardTooLow)
    );
}

/// A mining cycle is whole hours from one up to 2160 (`CoinIntrinsics.set_cycle_mining`).
#[test]
fn should_accept_a_cycle_of_one_to_2160_hours() {
    assert_eq!(CoinLimit::check_cycle_hours(2160.9), Ok(2160));
    assert_eq!(
        CoinLimit::check_cycle_hours(2161.0),
        Err(CoinLimit::CycleTooLong)
    );
    assert_eq!(
        CoinLimit::check_cycle_hours(0.5),
        Err(CoinLimit::CycleTooShort)
    );
}
