use crate::model::PreGenNpc;

/// What the content of a computer depends on.
pub struct Context<'a> {
    /// Public address of the network of the computer.
    pub public_ip: &'a str,
    /// Addresses of the networks of the mission (`NetworkLan.GetPublicIPsIdx`).
    pub group: &'a [String],
    /// Local addresses of the mission computers of this network (`NetworkLan.GetLocalIPsIdx`).
    pub local_ips: &'a [String],
    pub npcs: &'a [PreGenNpc],
    /// Local addresses of the mission computers of network `index` of the mission.
    pub remote_local_ips: &'a dyn Fn(usize) -> Vec<String>,
    /// Accounts of the computer with their passwords, root first.
    pub users: &'a [(String, String)],
    /// Game time in seconds (`ClockServer.GetFechaCompleta`).
    pub now: u64,
    /// Seed of the draws (`Router.GeneraLanPc`).
    pub seed: i32,
    /// The address of the zero-day rumor network (`ZeroDaySystem.GetIpAddress`), empty while there is none.
    pub zero_day_ip: &'a str,
    /// A seed for the objective (`WorldSeed.DeriveInt("procedural-objective", ...)`).
    pub objective_seed: i32,
}
