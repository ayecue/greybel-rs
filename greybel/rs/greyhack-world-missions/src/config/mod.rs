//! The templates procedural missions are built from (`MissionConfig.ProcMissionsConfig` and the classes it contains),
//! read from the game's `MissionProcedural` text. A missing field has the value its class gives it.

use serde::Deserialize;

macro_rules! numbered_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident = $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
        #[serde(try_from = "u8")]
        pub enum $name {
            #[default]
            $($variant = $value),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
        }

        impl TryFrom<u8> for $name {
            type Error = String;

            fn try_from(value: u8) -> Result<Self, String> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| *candidate as u8 == value)
                    .ok_or_else(|| format!("{} has no value {value}", stringify!($name)))
            }
        }
    };
}

numbered_enum! {
    /// `MissionConfig.Archetype`.
    Archetype {
        Sabotage = 0,
        Fraud = 1,
        Ransomware = 2,
        CorporateEspionage = 3,
        Extortion = 4,
        IdentityTheft = 5,
        EvidenceFabrication = 6,
        AccessSales = 7,
    }
}

numbered_enum! {
    /// `MissionConfig.ProceduralMissionSource`.
    Source {
        Hackshop = 0,
        Police = 1,
    }
}

numbered_enum! {
    /// `MissionConfig.Verb`.
    Verb {
        Recon = 0,
        CredentialPivot = 1,
        Analysis = 2,
        Recovery = 3,
    }
}

numbered_enum! {
    /// `MissionConfig.Modality`.
    Modality {
        ChatLog = 0,
        File = 1,
        CrossInfo = 2,
        Email = 3,
    }
}

numbered_enum! {
    /// `MissionConfig.CrossInfoPayloadType`.
    PayloadType {
        ChatLog = 0,
        File = 1,
    }
}

numbered_enum! {
    /// `MissionConfig.CrossInfoNextMode`.
    NextMode {
        CoexistWithNextBeat = 0,
        ReplaceNextBeat = 1,
    }
}

numbered_enum! {
    /// `MissionConfig.ClosureEntryType`.
    ClosureType {
        File = 0,
        ChatLog = 1,
    }
}

numbered_enum! {
    /// `MissionConfig.EvidenceFunctionType`.
    EvidenceType {
        NextNodeLocal = 0,
        NextNodeRemote = 1,
        NpcNameGlobal = 2,
        NpcNameLocal = 3,
        NextServiceLocal = 4,
        NextServiceRemote = 5,
        NextSystemLog = 6,
        ActualSystemLog = 7,
        RandomPublicIp = 8,
        RandomLocalIp = 9,
        NextNpcPassword = 10,
        RandomStringLocal = 11,
        RandomStringGlobal = 12,
        NpcNameRandom = 13,
        ClockOffset = 14,
        OctetPermutation = 15,
        TransferBalance = 16,
        NextNpcEncryptedPassword = 17,
        DigitChecksum = 18,
        FileTimeline = 19,
        SplitPayment = 20,
        BlockCoverage = 21,
        CircularLog = 22,
        RevisionBoundary = 23,
        HistoricalPassword = 24,
    }
}

mod archetype;
mod catalog;
mod chat_log_path_spec;
mod closure;
mod closure_entry;
mod companion_file_spec;
mod cross_info_spec;
mod defaults;
mod evidence_function;
mod file_option_spec;
mod heart;
mod modality;
mod proc_missions;
mod system_log_spec;
mod template_option;
mod verb;

pub use archetype::ArchetypeConfig;
pub use catalog::MissionCatalog;
pub use closure_entry::ClosureEntry;
pub use companion_file_spec::CompanionFileSpec;
pub use evidence_function::EvidenceFunction;
pub use file_option_spec::FileOptionSpec;
pub use proc_missions::ProcMissionsConfig;
pub use system_log_spec::SystemLogSpec;
pub use template_option::TemplateOption;
