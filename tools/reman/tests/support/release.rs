use reman::release::{Preparation, plan};
use reman::repository::Repository;

use super::Fixture;

/// A workspace `rs` where `app` uses `left` and `right`, which both use `core`.
pub fn diamond() -> Fixture {
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "core", &[], Some("library"))
        .crate_("rs", "left", &["core"], Some("library"))
        .crate_("rs", "right", &["core"], Some("library"))
        .crate_("rs", "app", &["left", "right"], Some("app"))
        .released()
        .commit();

    fixture
}

pub fn plan_of(repository: &Repository) -> plan::Plan {
    Preparation::new(repository, jiff::civil::date(2026, 2, 2))
        .unwrap()
        .plan
}

/// `id old -> new` for each release of the plan.
pub fn versions(repository: &Repository, plan: &plan::Plan) -> Vec<String> {
    plan.releases
        .iter()
        .map(|release| {
            format!(
                "{} {} -> {}",
                repository.nodes[release.node].id, release.previous, release.version
            )
        })
        .collect()
}

/// Writes the prepared release.
pub fn apply(repository: &Repository) {
    Preparation::new(repository, jiff::civil::date(2026, 2, 2))
        .unwrap()
        .edits
        .apply()
        .unwrap();
}
