//! Grey Hack failure text for `debug_tools` and the debug library payload.

use std::fmt::Write as _;

use super::metaxploit::weakness_text;
use crate::protocol::{DebugFailure, DebugScan, UnitTestReport, ZeroDayObjectKind};

/// Formats `MetaLibIntrinsics.debug_tools` and `DebugLibraryIntrinsics.payload` failures.
pub(super) fn debug_text(failure: &DebugFailure) -> String {
    match failure {
        DebugFailure::IncorrectPassword => "Error: Incorrect password".to_owned(),
        DebugFailure::CredentialsInUse => {
            "Error: These credentials are being used by another person.".to_owned()
        }
        DebugFailure::LoginBlocked => {
            "Error: The maximum number of login attempts has been reached. Access blocked."
                .to_owned()
        }
        DebugFailure::MissingFile => "error: file not found".to_owned(),
        DebugFailure::OutsideLib => {
            "Payload failed. The library must be found on the /lib path".to_owned()
        }
        DebugFailure::Patched => {
            "Error: The library is patched against this vulnerability.".to_owned()
        }
        DebugFailure::NoVulnerability => {
            "Failed: No vulnerabilities have been found in the specified memory area.".to_owned()
        }
        DebugFailure::UnreachableResource(path) => {
            format!("Error acquiring partial object file. Unable to access to resource: {path}")
        }
        DebugFailure::ScanPatchPresent => "No potential issues have been found".to_owned(),
        DebugFailure::TestPatchPresent => "Test failed: No potential issues have been found".to_owned(),
        DebugFailure::InvalidPatchFile => {
            "error: The file to apply the patch cannot be binary".to_owned()
        }
        DebugFailure::IncorrectPatch => "error: The patch code is not correct.".to_owned(),
        DebugFailure::PatchAlreadyPresent => "The patch had already been applied.".to_owned(),
        DebugFailure::UnitTestLockout => {
            "Error: Previous unit test failed. Access denied.".to_owned()
        }
        DebugFailure::IncorrectErrorLines => {
            "Test failed: No errors have been found in one or more of the provided lines, or not all lines with errors have been provided.\nSuspicious activity on the API. Access blocked.".to_owned()
        }
    }
}

/// Formats flawed library code with numbered lines (`DebugLibraryIntrinsics.scan_debuglib`).
pub(in crate::game) fn scan_text(scan: &DebugScan) -> String {
    let mut text = format!(
        "Potential problems have been found in the following code:\n\n{}",
        scan.description
    );

    if !scan.additional_descriptions.is_empty() {
        text.push_str("\n\n");
        text.push_str(&scan.additional_descriptions);
    }

    text.push_str("\n\n");

    for (index, line) in scan.code.split('\n').enumerate() {
        let _ = writeln!(text, "{:02}: {line}", index + 1);
    }

    text
}

/// Formats the zero-day a passing unit test reveals (`DebugLibraryIntrinsics.unit_testing`,
/// `VulnRandomMethods.PrintInfo`).
pub(in crate::game) fn unit_test_text(report: &UnitTestReport) -> String {
    let mut info = format!(
        "A potential issue has been found in address {}\nThe following partial objects can be obtained:\n\n",
        report.address
    );

    for object in &report.objects {
        let kind = match object.kind {
            ZeroDayObjectKind::Computer => "Computer",
            ZeroDayObjectKind::File => "File",
        };

        let _ = writeln!(
            info,
            "* {kind} [{}]: {}",
            object.access,
            object.methods.join(", ")
        );
    }

    format!(
        "Starting testing in selected lines...\n{info}\n\nNew vulnerability found: {}",
        weakness_text(&report.vulnerability)
    )
}
