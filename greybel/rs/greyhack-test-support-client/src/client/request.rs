use tokio::sync::oneshot;

use crate::TestClientError;
use crate::protocol::test_support::Reply;

pub(super) enum RequestKind {
    SetupPlayer,
    ConnectNetwork {
        public_ip: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    CreateNetwork {
        public_ip: String,
    },
    SetupDebug,
    SetupCtf {
        event_name: String,
    },
    Execute {
        source: String,
        filepath: String,
        basepath: String,
        program_name: String,
    },
    ExecuteInteractive {
        source: String,
        filepath: String,
        basepath: String,
        program_name: String,
        inputs: Vec<String>,
    },
    ExecuteUntilInput {
        source: String,
        filepath: String,
        basepath: String,
        program_name: String,
    },
    TypeInput {
        input: String,
    },
    ExecutePair {
        first_source: String,
        second_source: String,
        filepath: String,
        basepath: String,
        program_name: String,
    },
    QueryPlayer,
    QueryFile {
        path: String,
    },
    SetRouterKernelRequirement {
        local_ip: String,
        requirement: u8,
    },
    WriteRouterKernel {
        local_ip: String,
        content: String,
    },
    QueryNetwork,
    InstallLibrary {
        local_ip: String,
        name: String,
        reward: u8,
    },
    SetupBlockchain {
        coin_name: String,
        wallet_user: String,
    },
    SetupMail {
        user: String,
        password: String,
    },
    SetupWallet {
        user: String,
        password: String,
    },
    SetupForeignCoin {
        coin_name: String,
        user: String,
        password: String,
    },
    ForwardService {
        local_ip: String,
        public_port: u16,
        internal_port: u16,
        service: String,
    },
    LockAppliance {
        local_ip: String,
    },
    PrimeFlood {
        local_ip: String,
    },
    FreezeClock,
    FindNetwork {
        kind: String,
        device: String,
    },
    FindNewNetwork {
        kind: String,
        service: String,
    },
    DisableNetwork {
        public_ip: String,
        disabled: bool,
    },
    PromoteAdmin {
        public_ip: String,
    },
    FireNpc {
        public_ip: String,
        user: String,
    },
    RefreshNpcAccount {
        public_ip: String,
        user: String,
    },
    ConfigZeroDayInfo {
        public_ip: String,
    },
    SetPhoneAlarm {
        public_ip: String,
        user: String,
        hour: i64,
        minute: i64,
        delete_alarm: bool,
    },
    SetupTraffic,
    TrafficAccident {
        user: String,
        public_ip: String,
        police_ip: String,
    },
    ResetTrafficCooldown,
    HireRentalAdmin {
        public_ip: String,
    },
    SpawnNetwork {
        kind: String,
    },
    AllowFirewall {
        local_ip: String,
    },
    ForceAdminLogin {
        public_ip: String,
    },
    AdvanceClock {
        seconds: u64,
    },
    QueryExploits {
        library: String,
        version: String,
    },
    ProceduralMission {
        police: bool,
        index: u32,
    },
    BuyExploit {
        library: String,
        version: String,
        exploit: String,
        folder: String,
        compile: bool,
    },
    RebootPlayer {
        safe_mode: bool,
    },
    ReportLibraryUpdate {
        public_ip: String,
        library: String,
    },
    RentNetwork {
        public_ip: String,
        amount: i64,
    },
    RentalAdminService {
        public_ip: String,
        hire: bool,
    },
    RunPayments,
    SetPlayerMoney {
        money: f64,
    },
    SetupGuild(GuildSetup),
    QueryGuilds {
        company_ips: Vec<String>,
    },
    QueryTraces,
    QueryWordLists,
}

/// A guild the game gets: see `SetupGuildRpc`.
#[derive(Debug, Clone, Default)]
pub struct GuildSetup {
    pub name: String,
    pub bank_account: String,
    pub total_exp: u32,
    pub investment_perks: u32,
    pub join: bool,
    pub company_ip: String,
    pub crown: bool,
    pub shield_ip: String,
    pub admin_restore_perks: u32,
    pub admin_restore_off: bool,
}

pub(super) struct Request {
    pub kind: RequestKind,
    pub response: oneshot::Sender<Result<Reply, TestClientError>>,
}
