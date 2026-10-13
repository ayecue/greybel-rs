use greybel_test_support::future::block_on;
pub use greybel_test_support::world::subwallet;
use greyhack_behaviour::hash_password;
use greyhack_world::{CoinId, PlayerOptions, SubWallet, World};

pub fn ether(world: &World) -> CoinId {
    block_on(world.coin_by_name("Ether"))
        .expect("the world reads")
        .expect("Ether")
        .id
}

/// Adds a subwallet and its owning wallet when the fixture has not created it yet.
pub fn add_subwallet(world: &mut World, subwallet: &SubWallet) {
    block_on(async {
        if world
            .wallet_by_user(&subwallet.wallet_user)
            .await
            .expect("the world reads")
            .is_none()
        {
            world
                .add_wallet(&subwallet.wallet_user, "test", "0000", "test")
                .await
                .expect("the wallet is added");
        }

        let mut subwallet = subwallet.clone();

        subwallet.password_hash = hash_password("");
        world
            .add_subwallet(&subwallet, "")
            .await
            .expect("the subwallet is added");
    });
}

/// A new world whose Ether coin was created by the user `test` with the password `test`.
pub fn world_with_ether_created_by_test() -> World {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);
    let mut target = block_on(world.coin(coin))
        .expect("the world reads")
        .expect("Ether");

    target.creator_user = "test".to_owned();
    target.creator_password_hash = hash_password("test");
    block_on(world.save_coin(&target)).expect("the coin is saved");

    world
}

/// The minutes since the start of the month of the date `dd/Mon/yyyy - HH:mm`.
pub fn minutes(date: &str) -> u32 {
    let number = |range: std::ops::Range<usize>| date[range].parse::<u32>().expect("a number");

    (number(0..2) * 24 + number(14..16)) * 60 + number(17..19)
}
