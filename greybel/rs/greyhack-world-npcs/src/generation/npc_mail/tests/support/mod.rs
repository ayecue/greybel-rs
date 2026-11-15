use greyhack_behaviour::{CompanyAlign, JobRole, NetworkAdmin, WorkPerformance};
use mono_random::Random;

use super::super::{Administrator, Colleague, Delivery, Owner, contents};

pub(super) fn colleague(surname: &str, role: JobRole, performance: WorkPerformance) -> Colleague {
    Colleague {
        surname: surname.to_owned(),
        first_name: format!("First{surname}"),
        address: format!("{surname}@corp.com"),
        job_role: role,
        work_performance: performance,
    }
}

pub(super) fn network() -> Vec<Colleague> {
    vec![
        colleague("owner", JobRole::Sales, WorkPerformance::Normal),
        colleague("bad", JobRole::Sales, WorkPerformance::VeryPoor),
        colleague("good", JobRole::Sales, WorkPerformance::Excellent),
        colleague("boss", JobRole::Admin, WorkPerformance::VeryPoor),
    ]
}

pub(super) fn owner(network: &[Colleague]) -> Owner {
    Owner {
        colleague: network[0].clone(),
        break_start: 11 * 3_600 + 45 * 60,
        company_align: CompanyAlign::None,
        day_zero_contact: None,
    }
}

pub(super) fn boss(stats: NetworkAdmin) -> Administrator {
    Administrator {
        surname: "boss".to_owned(),
        stats,
    }
}

pub(super) fn weak_admin() -> NetworkAdmin {
    NetworkAdmin {
        intelligence: 2,
        pc_experience: 2,
        work_performance: WorkPerformance::VeryPoor,
        dead_or_fired: false,
    }
}

pub(super) fn text(key: &str) -> String {
    format!("<{key}>")
}

pub(super) fn run(
    seed: i32,
    owner: &Owner,
    network: &[Colleague],
    admin: Option<&Administrator>,
) -> Vec<Delivery> {
    contents(&mut Random::new(seed), owner, network, admin, None, &text)
}

pub(super) fn all(
    owner: &Owner,
    network: &[Colleague],
    admin: Option<&Administrator>,
) -> Vec<Vec<Delivery>> {
    (0..400)
        .map(|seed| run(seed, owner, network, admin))
        .collect()
}
