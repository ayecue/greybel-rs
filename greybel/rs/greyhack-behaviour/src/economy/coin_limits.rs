//! Limits of a coin's mining reward and cycle (`CoinIntrinsics.set_reward`, `set_cycle_mining`).

/// Maximum coins of one kind that can be mined, and the most a mining reward or a sub-wallet balance
/// can reach (`BlockchainSystem.MAX_COINS`; `CoinIntrinsics.set_reward` states the same limit).
pub const MAX_COINS: i32 = 100_000_000;
/// Maximum mining cycle in hours (`CoinIntrinsics.set_cycle_mining`).
pub const MAX_CYCLE_HOURS: i32 = 2160;

/// Why a mining reward or cycle is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoinLimit {
    /// The reward exceeds [`MAX_COINS`].
    RewardTooHigh,
    /// The reward is under one coin.
    RewardTooLow,
    /// The cycle exceeds [`MAX_CYCLE_HOURS`].
    CycleTooLong,
    /// The cycle is under one hour.
    CycleTooShort,
}

impl CoinLimit {
    /// Validates a mining `reward` and returns its integer part (`CoinIntrinsics.set_reward`).
    ///
    /// # Errors
    ///
    /// Returns [`Self::RewardTooHigh`] above [`MAX_COINS`] and [`Self::RewardTooLow`] below 1.
    pub fn check_reward(reward: f64) -> Result<i32, Self> {
        if reward > f64::from(MAX_COINS) {
            return Err(Self::RewardTooHigh);
        }

        let reward = Self::int_value(reward);

        if reward < 1 {
            return Err(Self::RewardTooLow);
        }

        Ok(reward)
    }

    /// Validates a mining cycle and returns its integer hours (`CoinIntrinsics.set_cycle_mining`).
    ///
    /// # Errors
    ///
    /// Returns [`Self::CycleTooLong`] above [`MAX_CYCLE_HOURS`] and [`Self::CycleTooShort`] below 1.
    pub fn check_cycle_hours(cycle_hours: f64) -> Result<i32, Self> {
        let cycle_hours = Self::int_value(cycle_hours);

        if cycle_hours > MAX_CYCLE_HOURS {
            return Err(Self::CycleTooLong);
        }

        if cycle_hours < 1 {
            return Err(Self::CycleTooShort);
        }

        Ok(cycle_hours)
    }

    /// Converts `number` with C# `(int)` semantics, returning `int.MinValue` for NaN and out-of-range
    /// values (`Value.IntValue`).
    #[expect(
        clippy::cast_possible_truncation,
        reason = "Value.IntValue truncates like C# (int) after its range checks"
    )]
    fn int_value(number: f64) -> i32 {
        if number.is_nan() || number >= 2_147_483_648.0 || number <= -2_147_483_649.0 {
            i32::MIN
        } else {
            number as i32
        }
    }
}
