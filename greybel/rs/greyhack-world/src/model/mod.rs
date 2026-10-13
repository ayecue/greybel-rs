//! Game-world data types with entity-local methods. Cross-entity behavior lives in
//! [`crate::systems`].

/// Numeric entity ID representation for protocols and external callers.
macro_rules! id_numbers {
    ($($id:ident),*) => {$(
        impl $id {
            /// Converts `number` to a typed entity ID.
            #[must_use]
            pub const fn new(number: u32) -> Self {
                Self(number)
            }

            /// Converts a typed ID to its numeric representation.
            #[must_use]
            pub const fn number(self) -> u32 {
                self.0
            }
        }
    )*};
}

use greyhack_behaviour::{
    Command, DeviceKind, FileKind, HardwareKind, LibraryKind, NetworkKind, Requirement, Reward,
    ServiceId, SubscriptionKind,
};

/// Reads an [`enum_def!`](greyhack_behaviour::enum_def) variant from its database integer.
macro_rules! sql_enum {
    ($($name:ident),+ $(,)?) => {$(
        impl crate::storage::sql::FromSql for $name {
            fn from_sql(value: turso::Value) -> turso::Result<Self> {
                let code = <i64 as crate::storage::sql::FromSql>::from_sql(value)?;

                Self::try_from(code)
                    .map_err(|error| turso::Error::ConversionFailure(error.to_string()))
            }
        }
    )+};
}

pub(crate) mod apt;
pub(crate) mod computer;
mod ctf;
mod debug;
pub(crate) mod economy;
mod exploit;
mod guild;
pub(crate) mod mail;
pub(crate) mod network;
mod npc;
mod person;
mod trace;

pub use apt::{AptFailure, Package, PackageEntry, Repository};
pub(crate) use computer::account_files::{
    add_home_folders, etc_folder, home_folders, passwd_file_draft, passwd_is_reusable, passwd_text,
};
pub use computer::camera::{CameraFeed, CameraNpc};
pub use computer::failure::{FileFailure, NetCardFailure, ProcessFailure, UserFailure};
pub(crate) use computer::filesystem::DraftKind;
pub use computer::filesystem::{CopyReport, FileSystemDraft, Node, NodeDraft, NodeId, NodeKind};
pub use computer::startup::StartupFailure;
pub use computer::user::User;
pub use computer::{
    Computer, ComputerDraft, ComputerId, Cpu, Gpu, HardDisk, Hardware, LanPlace, MiningHardware,
    Motherboard, NetCard, Part, Port, PortForward, PowerSupply, Process, Ram, SavedNetwork,
    SavedWifiFailure,
};
pub(crate) use computer::{STANDARD_DISK_MB, STANDARD_RAM_MB};
pub use ctf::{CtfAccount, CtfEvent, CtfFailure, CtfPlayer};
pub use debug::{
    DebugAccount, DebugChallenge, DebugLibraryState, ZeroDay, ZeroDayObject, ZeroDayObjectKind,
};
pub use economy::blockchain::{
    BankAccount, BankLayout, BankSubscription, BankTransaction, Coin, CoinId, CoinLayout,
    EconomyLayout, Offer, OfferLayout, PriceRecord, Side, SubWallet, SubWalletLayout, Transaction,
    WalletLayout,
};
pub use economy::failure::{
    BankTransferFailure, MarketFailure, MineJobFailure, PurchaseFailure, Trade, TransferFailure,
};
pub use economy::wallet::{Wallet, WalletId};
pub use exploit::{Exploit, ExploitOffer};
pub use greyhack_behaviour::{
    AlarmConfig, CompanyAlign, JobRole, NetworkAdmin, NpcSchedule, SmartApplianceSettings,
    WorkPerformance,
};
pub use guild::{
    AdminRestoreFailure, CompanyLayout, CrownShieldFailure, CrownState, Guild, GuildAbsence,
    GuildDraft, GuildFailure, GuildId, GuildLayout, GuildsLayout, Stake,
};
pub use mail::{Email, MailAccount};
pub use network::capture::WifiCapture;
pub use network::failure::{
    ConnectionFailure, LanFailure, LoginFailure, NetUseFailure, RshellFailure, ServiceFailure,
    SmtpFailure,
};
pub use network::layout::{DeviceLayout, ForwardLayout, LanNodeLayout, NetworkLayout};
pub use network::records::{
    Crime, FacePart, Grade, Offense, Record, RecordDatabase, RecordKind, Subject, Vector,
};
pub use network::router::{Contact, NetworkDraft, ReverseShell, Router, RouterDraft, RouterId};
pub use network::service_config::{
    DEFAULT_DECODE_PATH, DEFAULT_ENCODE_PATH, HttpConfig, RepositoryConfig, SoftwareInfo,
    SshdConfig,
};
pub use network::session::NetUseSession;
pub use network::sniffer::SnifferPacket;
pub use network::traffic::{LocatedVehicle, TrafficNet};
pub use network::weakness::{Library, LibraryId, MemoryZone, Weakness};
pub use network::{Network, NetworkId};
pub use npc::{Npc, NpcEvent};
pub use person::{Gender, Person, PersonId};
pub use trace::{ActiveTrace, PassiveState, TraceId, TraceKind};

id_numbers!(
    ComputerId, RouterId, NodeId, CoinId, WalletId, PersonId, NetworkId, LibraryId
);

sql_enum!(
    Command,
    DeviceKind,
    FileKind,
    Gender,
    HardwareKind,
    LibraryKind,
    NetworkKind,
    Requirement,
    Reward,
    ServiceId,
    SubscriptionKind
);
