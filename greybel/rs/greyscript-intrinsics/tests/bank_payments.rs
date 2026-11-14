use greybel_test_support::future::block_on;
use greyhack_world::{PlayerOptions, World};

use crate::support::bank_payments::SCRIPT;
use crate::support::scripts::run_on_host;

/// Hiring an administrator is paid from the player's bank account: the first month leaves 500 of 3000, so the second
/// hire after the first one is removed cannot be paid (`BankHelperServer.HireAdminRentalServerRpc`,
/// `BankHelperServer.PagarCompra`).
#[test]
fn should_refuse_to_hire_an_admin_the_bank_account_cannot_pay() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let (output, error, world) = run_on_host(SCRIPT, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n1\nYou don't have any administrator hired on this network.\nThe purchase could not be completed. Insufficient funds"
    );
    assert_eq!(
        block_on(world.modules.economy.bank.bank_account("test"))
            .unwrap()
            .unwrap()
            .money,
        500.0
    );
}

/// A player with no bank account cannot hire.
#[test]
fn should_refuse_to_hire_an_admin_without_a_bank_account() {
    let world = block_on(World::new(&PlayerOptions {
        bank_number: None,
        ..PlayerOptions::default()
    }))
    .expect("a world");
    let (output, error, _) = run_on_host(SCRIPT, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "You must register a bank account in order to buy\nYou don't have any administrator hired on this network.\nYou don't have any administrator hired on this network.\nYou must register a bank account in order to buy"
    );
}
