use greyhack_world::BankTransferFailure;

use crate::support::economy_bank::bank_world;

#[tokio::test]
async fn should_charge_a_bank_transfer_fee_and_cap_both_histories() {
    let world = bank_world(100_000.0, 0.0).await;

    for amount in (1..=16).map(|step| step * 100) {
        assert_eq!(
            world
                .modules
                .economy
                .bank
                .bank_transfer("source", "pw", "destination", amount)
                .await
                .unwrap(),
            Ok(())
        );
    }

    let source = world
        .modules
        .economy
        .bank
        .bank_account("source")
        .await
        .unwrap()
        .unwrap();
    let destination = world
        .modules
        .economy
        .bank
        .bank_account("destination")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(source.money, 86_400.0);
    assert_eq!(destination.money, 11_560.0);
    assert_eq!(source.transactions.len(), 30);
    assert_eq!(destination.transactions.len(), 16);
    assert_eq!(source.transactions[0].amount, -30);
    assert_eq!(source.transactions[0].reason, "Transaction fee");
    assert_eq!(source.transactions[1].amount, -170);
    assert_eq!(source.transactions[1].reason, "Money transfer");
    assert_eq!(destination.transactions[0].amount, 85);
    assert_eq!(destination.transactions[0].account, "destination");
    assert!(
        source
            .transactions
            .iter()
            .all(|transaction| transaction.success)
    );
}

#[tokio::test]
async fn should_validate_a_bank_transfer_before_changing_money() {
    let world = bank_world(100.0, 0.0).await;

    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_transfer("source", "pw", "source", 100)
            .await
            .unwrap()
            .unwrap_err(),
        BankTransferFailure::SameAccount
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_transfer("source", "wrong", "destination", 100)
            .await
            .unwrap()
            .unwrap_err(),
        BankTransferFailure::WrongPassword
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_transfer("source", "pw", "missing", 100)
            .await
            .unwrap()
            .unwrap_err(),
        BankTransferFailure::UnknownAccount("missing".to_owned())
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_transfer("source", "pw", "destination", 0)
            .await
            .unwrap()
            .unwrap_err(),
        BankTransferFailure::InsufficientFunds
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_transfer("source", "pw", "destination", 101)
            .await
            .unwrap()
            .unwrap_err(),
        BankTransferFailure::InsufficientFunds
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .money,
        100.0
    );
    assert!(
        world
            .modules
            .economy
            .bank
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .transactions
            .is_empty()
    );

    // The gross amount is affordable even though the destination receives only the net amount.
    world
        .modules
        .economy
        .bank
        .bank_transfer("source", "pw", "destination", 100)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .money,
        0.0
    );
    assert_eq!(
        world
            .modules
            .economy
            .bank
            .bank_account("destination")
            .await
            .unwrap()
            .unwrap()
            .money,
        85.0
    );
}
