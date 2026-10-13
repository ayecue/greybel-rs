use greyhack_behaviour::RankedCompany;

pub fn company(points: i32, percent: f32) -> RankedCompany {
    RankedCompany {
        points,
        owner_percent: percent,
        ddos_penalty: false,
        admin_penalty_percent: 0,
    }
}
