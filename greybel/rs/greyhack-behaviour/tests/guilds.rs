use greyhack_behaviour::{
    ADMIN_ABSENCE_PENALTY_PERCENT, AdminAbsence, AdminArrival, NetworkKind, RankedGuild, add_days,
    admin_absent, admin_arrives, crown_leader, effective_max_admin_restore_disabled_companies,
    effective_max_invested_companies, guild_points, is_listed_on_stock_market, leaderboard_points,
    majority_owner, max_admin_restore_companies_for_level, max_admin_restore_companies_for_perks,
    max_invested_companies_for_level, max_stocks, next_day, reputation_level, sort_leaderboard,
};

use crate::support::guilds::company;

#[test]
fn should_limit_the_companies_a_guild_turns_admin_restore_off_for() {
    assert_eq!(max_admin_restore_companies_for_perks(0), 0);
    assert_eq!(max_admin_restore_companies_for_perks(3), 3);
    assert_eq!(max_admin_restore_companies_for_perks(9), 5);
    assert_eq!(max_admin_restore_companies_for_level(4), 0);
    assert_eq!(max_admin_restore_companies_for_level(8), 2);
    assert_eq!(
        effective_max_admin_restore_disabled_companies(0, 275 * 225),
        0
    );
    assert_eq!(
        effective_max_admin_restore_disabled_companies(3, 275 * 25),
        1
    );
    assert_eq!(
        effective_max_admin_restore_disabled_companies(9, 275 * 225),
        5
    );
}

#[test]
fn should_level_a_guild_by_the_square_root_of_its_experience() {
    assert_eq!(reputation_level(0), 0);
    assert_eq!(reputation_level(274), 0);
    assert_eq!(reputation_level(275), 1);
    assert_eq!(reputation_level(275 * 25), 5);
    assert_eq!(reputation_level(275 * 25 - 1), 4);
}

#[test]
fn should_let_a_guild_invest_from_level_five_up_to_its_licences() {
    assert_eq!(max_invested_companies_for_level(4), 0);
    assert_eq!(max_invested_companies_for_level(5), 1);
    assert_eq!(max_invested_companies_for_level(8), 2);
    assert_eq!(max_invested_companies_for_level(15), 5);
    assert_eq!(effective_max_invested_companies(0, 275 * 25), 0);
    assert_eq!(effective_max_invested_companies(3, 275 * 25), 1);
    assert_eq!(effective_max_invested_companies(9, 275 * 225), 5);
}

#[test]
fn should_find_a_majority_owner_above_half_and_the_first_of_equals() {
    assert_eq!(majority_owner(100, [("a", 50)]), None);
    // The percent is single-precision arithmetic, as the game's.
    assert_eq!(
        majority_owner(100, [("a", 51)]),
        Some(("a", 51.0_f32 / 100.0 * 100.0))
    );
    assert_eq!(
        majority_owner(100, [("a", 30), ("b", 60)]),
        Some(("b", 60.0_f32 / 100.0 * 100.0))
    );
    assert_eq!(majority_owner(0, [("a", 1)]), None);
}

#[test]
fn should_round_guild_points_from_the_unrounded_weighted_product() {
    assert_eq!(guild_points(&[company(45, 70.0)], 1), 31);
}

#[test]
fn should_issue_shares_by_network_kind() {
    assert_eq!(max_stocks(NetworkKind::Police), 1_000_000);
    assert_eq!(max_stocks(NetworkKind::HackShop), 500_000);
    assert_eq!(max_stocks(NetworkKind::Supermarket), 100_000);
    assert!(is_listed_on_stock_market(NetworkKind::Garage));
    assert!(is_listed_on_stock_market(NetworkKind::ComputerShop));
    assert!(!is_listed_on_stock_market(NetworkKind::Bank));
}

#[test]
fn should_score_a_company_by_its_market_value_and_work_performance() {
    // 100000 shares at 5 with normal work (2): 500000 / 1000 = 500.
    assert_eq!(leaderboard_points(100_000, 5, 2), 500);
    // Excellent work (5) raises the points by 30 percent.
    assert_eq!(leaderboard_points(100_000, 5, 5), 650);
    // Very poor work (0) lowers them by 20 percent.
    assert_eq!(leaderboard_points(100_000, 5, 0), 400);
    // A price below one counts as one, and halves round to even.
    assert_eq!(leaderboard_points(100_000, 0, 2), 100);
    assert_eq!(leaderboard_points(5, 1, 2), 0);
    assert_eq!(leaderboard_points(15, 1, 2), 0);
    assert_eq!(leaderboard_points(25, 1, 2), 0);
}

#[test]
fn should_count_only_the_companies_within_the_guilds_limit() {
    let companies = [company(1000, 100.0), company(500, 100.0)];

    assert_eq!(guild_points(&companies, 1), 1000);
    assert_eq!(guild_points(&companies, 2), 1500);
    assert_eq!(guild_points(&companies, 0), 0);
}

#[test]
fn should_scale_points_by_the_owned_share_and_the_penalties() {
    let mut company = company(1000, 60.0);

    assert_eq!(guild_points(&[company.clone()], 1), 600);

    company.ddos_penalty = true;

    assert_eq!(guild_points(&[company.clone()], 1), 480);

    company.admin_penalty_percent = 50;

    assert_eq!(guild_points(&[company.clone()], 1), 240);

    company.admin_penalty_percent = 150;

    assert_eq!(guild_points(&[company], 1), 0);
}

#[test]
fn should_rank_by_points_then_name_and_leave_a_tie_without_a_leader() {
    let mut ranking = vec![
        RankedGuild {
            name: "b".to_owned(),
            total_points: 5,
        },
        RankedGuild {
            name: "a".to_owned(),
            total_points: 5,
        },
        RankedGuild {
            name: "c".to_owned(),
            total_points: 9,
        },
    ];

    sort_leaderboard(&mut ranking);

    assert_eq!(ranking[0].name, "c");
    assert_eq!(ranking[1].name, "a");
    assert_eq!(
        crown_leader(&ranking).map(|guild| guild.name.as_str()),
        Some("c")
    );

    ranking[0].total_points = 5;

    assert_eq!(crown_leader(&ranking), None);
    assert_eq!(
        crown_leader(&ranking[..1]).map(|guild| guild.total_points),
        Some(5)
    );
    assert_eq!(crown_leader(&[]), None);
}

#[test]
fn should_penalize_a_missing_administrator_and_recover_ten_points_a_day() {
    let missing = admin_absent(None, 10).expect("a new record");

    assert_eq!(missing.percent, ADMIN_ABSENCE_PENALTY_PERCENT);
    assert!(!missing.recovering);
    assert_eq!(admin_absent(Some(missing), 11), None);

    let AdminArrival::Recover(recovering) = admin_arrives(Some(missing), 12) else {
        panic!("a recovery starts");
    };

    assert_eq!(recovering.percent_on(12), 50);
    assert_eq!(recovering.percent_on(14), 30);
    assert_eq!(recovering.percent_on(30), 0);
    assert_eq!(admin_arrives(Some(recovering), 13), AdminArrival::Same);
    assert_eq!(admin_arrives(None, 13), AdminArrival::Same);

    let absent_again = admin_absent(Some(recovering), 14).expect("a changed record");

    assert_eq!(absent_again.percent, 50);
    assert!(!absent_again.recovering);
    assert_eq!(
        admin_arrives(
            Some(AdminAbsence {
                percent: 0,
                recovery_day: None,
                recovering: false
            }),
            1
        ),
        AdminArrival::Remove
    );
}

#[test]
fn should_count_days_from_midnight() {
    assert_eq!(add_days(100, 2), 100 + 2 * 86_400);
    assert_eq!(next_day(86_400 * 3 + 5), 86_400 * 4);
}
