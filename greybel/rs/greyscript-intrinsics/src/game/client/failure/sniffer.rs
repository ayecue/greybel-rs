//! Grey Hack text for packets captured by `sniffer`.

use std::fmt::Write as _;

use crate::protocol::{FileFailure, SniffedPacket};

/// Formats a captured packet for `sniffer` (`HelperScript.GetPendingSniffers`).
pub(in crate::game) fn sniffed_text(packet: &SniffedPacket) -> String {
    let mut text = format!(
        "Incoming data captured.\n\n<b>Source:</b> {}\n<b>Destination:</b> {}\n<b>Protocol:</b> {}\n<b>USER:</b> {}\n<b>PASSWORD:</b> {}\n",
        packet.source, packet.destination, packet.protocol, packet.user, packet.password
    );

    if packet.encrypted {
        text = text.replace("PASSWORD", "PASSWORD[encrypted]");
    }

    if let Some(failure) = &packet.encoder_failure {
        let reason = match failure {
            FileFailure::MissingFolder(folder) => format!("{folder}: folder not found"),
            FileFailure::NoReadPermission(path) => format!("can't open {path}. Permission denied."),
            _ => "computer not found".to_owned(),
        };

        let _ = write!(text, "\nWarning: Unable to create encode.src\n{reason}");
    }

    text
}
