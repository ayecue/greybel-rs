use crate::support::program::{
    BlockchainCreateWallet, BlockchainGetCoin, BlockchainLoginWallet, CoinCreateSubwallet,
    CoinGetSubwallet, CoinLogin, ComputerFile, ComputerTouch, IncludedLibrary, LaunchOutcome,
    ObjectId, Outcome, PlayerIncludeLib, ProgramSession, ShellLaunch, WalletGetPin, WalletLogin,
};
use crate::support::{Runtime, player};

/// Returns the file object for `notes.txt` created in the player's home by `runtime`'s program `session`.
pub async fn notes(runtime: &Runtime, session: &ProgramSession) -> ObjectId {
    let home = format!("/home/{}", player());
    let touched = runtime
        .channel
        .send(&ComputerTouch {
            program: session.program,
            computer: session.shell,
            folder: home.as_str().into(),
            name: "notes.txt".into(),
        })
        .expect("the change is sent")
        .await
        .expect("the game server responds");

    assert!(matches!(touched, Outcome::True | Outcome::Failure(_)));

    runtime
        .channel
        .send(&ComputerFile {
            program: session.program,
            computer: session.shell,
            path: format!("{home}/notes.txt").into(),
        })
        .expect("the lookup is sent")
        .await
        .expect("the game server responds")
        .expect("the file is there")
}

/// Launches `path` from the program `parent`, as `shell.launch` does.
pub async fn launch(runtime: &Runtime, parent: &ProgramSession, path: &str) -> LaunchOutcome {
    runtime
        .channel
        .send(&ShellLaunch {
            program: parent.program,
            shell: parent.shell,
            path: path.into(),
            params: Vec::new().into(),
            current: parent.home.clone().into(),
        })
        .expect("the launch is sent")
        .await
        .expect("the game server responds")
}

/// The metaxploit library object of the player's computer, for the program `session`.
pub async fn metaxploit(runtime: &Runtime, session: &ProgramSession) -> ObjectId {
    let library = runtime
        .channel
        .send(&PlayerIncludeLib {
            program: session.program,
            path: "/lib/metaxploit.so".into(),
        })
        .expect("the include is sent")
        .await
        .expect("the game server responds");
    let IncludedLibrary::Metaxploit(metaxploit) = library else {
        panic!("the player has metaxploit.so");
    };

    metaxploit
}

/// Returns the Ether coin object, logged in with the test wallet the program `session` creates.
pub async fn ether(runtime: &Runtime, session: &ProgramSession) -> ObjectId {
    let library = runtime
        .channel
        .send(&PlayerIncludeLib {
            program: session.program,
            path: "/lib/blockchain.so".into(),
        })
        .expect("the include is sent")
        .await
        .expect("the game server responds");
    let IncludedLibrary::Blockchain(blockchain) = library else {
        panic!("the player has blockchain.so");
    };

    runtime
        .channel
        .send(&BlockchainCreateWallet {
            program: session.program,
            blockchain,
            user: Some("test".into()),
            password: Some("test".into()),
        })
        .expect("the wallet is sent")
        .await
        .expect("the game server responds");

    let login = runtime
        .channel
        .send(&BlockchainGetCoin {
            program: session.program,
            blockchain,
            user: Some("test".into()),
            password: Some("test".into()),
            coin: Some("Ether".into()),
        })
        .expect("the login is sent")
        .await
        .expect("the game server responds");
    let CoinLogin::Coin(coin) = login else {
        panic!("test created Ether: {login:?}");
    };

    coin
}

/// Creates the sub-wallets `users` of the coin `Ether`, whose creator is `test`/`test`, for the program `session`, as
/// `blockchain.so`'s `create_wallet`, `get_coin`, `create_subwallet` and `get_subwallet` do.
pub async fn sub_wallets(
    runtime: &Runtime,
    session: &ProgramSession,
    users: &[&str],
) -> Vec<ObjectId> {
    let coin = ether(runtime, session).await;
    let library = runtime
        .channel
        .send(&PlayerIncludeLib {
            program: session.program,
            path: "/lib/blockchain.so".into(),
        })
        .expect("the include is sent")
        .await
        .expect("the game server responds");
    let IncludedLibrary::Blockchain(blockchain) = library else {
        panic!("the player has blockchain.so");
    };
    let login = runtime
        .channel
        .send(&BlockchainLoginWallet {
            program: session.program,
            blockchain,
            user: Some("test".into()),
            password: Some("test".into()),
        })
        .expect("the wallet login is sent")
        .await
        .expect("the game server responds");
    let WalletLogin::Wallet(wallet) = login else {
        panic!("the wallet created for Ether opens: {login:?}");
    };
    let pin = runtime
        .channel
        .send(&WalletGetPin {
            program: session.program,
            wallet,
        })
        .expect("the PIN is sent")
        .await
        .expect("the game server responds")
        .expect("the wallet has a PIN");
    let mut sub_wallets = Vec::new();

    for user in users {
        let created = runtime
            .channel
            .send(&CoinCreateSubwallet {
                program: session.program,
                coin,
                wallet: "test".into(),
                pin: pin.clone().into(),
                user: (*user).into(),
                password: "pw".into(),
            })
            .expect("the sub-wallet is sent")
            .await
            .expect("the game server responds");

        assert_eq!(created, Outcome::True, "{user}'s sub-wallet is created");

        let sub_wallet = runtime
            .channel
            .send(&CoinGetSubwallet {
                program: session.program,
                coin,
                user: (*user).into(),
            })
            .expect("the lookup is sent")
            .await
            .expect("the game server responds")
            .expect("the sub-wallet exists");

        sub_wallets.push(sub_wallet);
    }

    sub_wallets
}
