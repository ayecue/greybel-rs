//! The strikes of a trace and the game over: what the third strike does to the player's computer, their payments, the
//! shares they own and their guild (`PlayerHelperServer.ActiveGameOver`, `Database.GameOver`). The game's strike is
//! given by its own `ActiveGameOver` after its own `Database.GameOverStrike`, the copy's by the end of an active trace.
//! The guild is compared by experience and membership, including an owner remaining a member.

use crate::support::parity_game_over::strike;

/// The third strike deletes the player's computer, ends their bank fee, gives the shares they hold back to the company
/// and costs their guild experience. The rental charge and list stay, its networks are disabled and their home flags
/// reset, and only the bank payment is removed (`Database.GameOver`, `BankSubs.RemovePayments`).
#[tokio::test]
async fn should_match_the_game_when_the_third_strike_ends_the_game() {
    strike(3).await;
}

/// The first strike changes none of this (`PlayerHelperServer.ActiveGameOver`).
#[tokio::test]
async fn should_match_the_game_when_the_first_strike_is_given() {
    strike(1).await;
}
