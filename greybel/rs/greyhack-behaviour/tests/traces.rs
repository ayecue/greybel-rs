use greyhack_behaviour::{
    LogAction, NetworkKind, SystemLog, active_trace_seconds, passive_trace_delay_ms,
    should_start_tracking,
};

#[test]
fn should_trace_shells_removals_and_downloads_on_sensitive_networks() {
    for kind in [
        NetworkKind::Police,
        NetworkKind::Bank,
        NetworkKind::HackShop,
        NetworkKind::Neurobox,
    ] {
        for action in [LogAction::Shell, LogAction::Remove, LogAction::Download] {
            assert!(
                should_start_tracking(kind, 1, action),
                "{kind:?} {action:?}"
            );
        }

        assert!(!should_start_tracking(kind, 1, LogAction::Open));
        assert!(!should_start_tracking(kind, 1, LogAction::Upload));
    }
}

#[test]
fn should_trace_removals_and_downloads_elsewhere_and_shells_of_a_skilled_administrator() {
    assert!(should_start_tracking(
        NetworkKind::Supermarket,
        1,
        LogAction::Remove
    ));
    assert!(should_start_tracking(
        NetworkKind::Supermarket,
        1,
        LogAction::Download
    ));
    assert!(!should_start_tracking(
        NetworkKind::Supermarket,
        7,
        LogAction::Shell
    ));
    assert!(should_start_tracking(
        NetworkKind::Supermarket,
        8,
        LogAction::Shell
    ));
    assert!(!should_start_tracking(
        NetworkKind::Supermarket,
        8,
        LogAction::Bounce
    ));
}

#[test]
fn should_trace_only_removals_on_residential_networks_with_a_skilled_administrator() {
    assert!(!should_start_tracking(
        NetworkKind::Residential,
        4,
        LogAction::Remove
    ));
    assert!(should_start_tracking(
        NetworkKind::Residential,
        5,
        LogAction::Remove
    ));
    assert!(!should_start_tracking(
        NetworkKind::Residential,
        5,
        LogAction::Download
    ));
    assert!(!should_start_tracking(
        NetworkKind::Residential,
        7,
        LogAction::Shell
    ));
    assert!(should_start_tracking(
        NetworkKind::Residential,
        8,
        LogAction::Shell
    ));
}

#[test]
fn should_last_longer_with_more_bounces_and_a_less_skilled_administrator() {
    assert_eq!(active_trace_seconds(1, 9), 100.0);
    assert_eq!(active_trace_seconds(3, 10), 270.0);
    assert!(active_trace_seconds(1, 1) > active_trace_seconds(1, 2));
    assert_eq!(passive_trace_delay_ms(9), 100_000);
    assert_eq!(passive_trace_delay_ms(0), 900_000);
}

#[test]
fn should_find_the_first_bounce_entry_of_the_player_and_their_token() {
    let entry = |bounce: &str, player: &str, token: &str| {
        format!(
            r#"{{"action":4,"ip":"{bounce}-origin","puerto":0,"fecha":"","file":"","bounceIp":"{bounce}","playerNetID":"{player}","tutorial":false,"tokenTrace":"{token}"}}"#
        )
    };
    let content = format!(
        r#"{{"contentLog":[{},{},{}]}}"#,
        entry("1.1.1.1", "other", "t"),
        entry("1.1.1.1", "me", "old"),
        entry("1.1.1.1", "me", "t")
    );
    let log = SystemLog::parse(&content).expect("a log");

    let found = log.ini_bounce("1.1.1.1", "me", "t").expect("an entry");

    assert_eq!(found.token_trace, "t");
    assert_eq!(found.player_net_id, "me");
    assert!(log.ini_bounce("2.2.2.2", "me", "t").is_none());
}
