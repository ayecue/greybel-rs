#[path = "support/mod.rs"]
mod support;

use support::Lines;

use super::{FloodWarning, print_line, start_text};
use crate::protocol::{
    AptUpdateLine, CryptoFailure, DebugFailure, ExploitLog, Failure, LaunchFailure, LoginFailure,
    NetUseFailure, PrintLine,
};

#[test]
fn should_warn_about_crownshield_without_naming_a_super_administrator() {
    let text = start_text("1.2.3.4", &FloodWarning::CrownShield);

    assert!(text.contains("Press Ctrl+C to stop\nWarning: DDoS traffic blocked by CrownShield."));
    assert!(!text.contains("Super administrator"));
}

#[test]
fn should_print_no_warning_for_a_plain_flood() {
    assert_eq!(
        start_text("1.2.3.4", &FloodWarning::None),
        "Flooding 1.2.3.4...\nPress Ctrl+C to stop"
    );
}

#[test]
fn should_render_every_print_line_variant() {
    let lines = Lines::default();
    let prints = [
        PrintLine::AptUpdate(AptUpdateLine::Update("1.2.3.4".to_owned())),
        PrintLine::CaptureHandshake {
            bssid: "AA:BB".to_owned(),
        },
        PrintLine::CaptureProgress {
            packets: 12,
            acks: 3,
        },
        PrintLine::CaptureSaveFailure(Failure::Crypto(CryptoFailure::IncorrectPassword)),
        PrintLine::KeyRecoveryFailure(Failure::Crypto(CryptoFailure::CorruptedCapture)),
        PrintLine::PayloadFailure(Failure::Debug(DebugFailure::OutsideLib)),
        PrintLine::ExploitLog(ExploitLog::MissingWeakness),
        PrintLine::NetUseFailure {
            failure: NetUseFailure::InvalidAddress,
            address: "bad".to_owned(),
        },
        PrintLine::HiddenZonesWarning,
        PrintLine::FloodStart {
            address: "1.2.3.4".to_owned(),
            warning: FloodWarning::SuperAdmin,
        },
        PrintLine::FloodInterrupted,
        PrintLine::LoginFailure(LoginFailure::IncorrectPassword),
        PrintLine::EncryptionEnabled,
        PrintLine::LaunchFailure(Failure::Launch(LaunchFailure::NoPermission)),
    ];

    for line in &prints {
        print_line(&lines, line);
    }

    assert_eq!(
        lines.0.into_inner(),
        [
            "Updating: 1.2.3.4",
            "Waiting for beacon frame (BSSID: AA:BB)\nSending Authentication Request (Open system) [ACK]\nAuthentication successful\nSending Association Request\nAssociation successful :-)",
            "Read 12 packets (got 3 ACKs)",
            "decrypt: Incorrect password",
            "Error: filecap corrupted!",
            "Payload failed. The library must be found on the /lib path",
            "Exploit failed. Vulnerability not found.",
            "Invalid ip address: bad",
            "**Warning: One or more memory zones with possible vulnerabilities could not be scanned. Update your Metaxploit library.**",
            "Flooding 1.2.3.4...\nPress Ctrl+C to stop\nWarning: Super administrator detected. Additional DDoS traffic required.",
            "remote connection interrupted",
            "can't connect: incorrect user/password",
            "Encryption enabled...",
            "Can't launch program. Permission denied.",
        ]
    );
}
