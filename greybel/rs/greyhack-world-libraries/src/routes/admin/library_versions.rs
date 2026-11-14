//! Releasing library versions and reporting weaknesses (`admin.so`).

use greyhack_behaviour::LibraryKind;

use crate::WeaknessReportFailure;
use crate::routes::Result;
use crate::routes::protocol::{AdminFailure, Failure, LibraryRelease, Outcome};

/// Releases the next version of the library file `library` (`VersionsControl.ForceInitialVersionGeneration`); a
/// library the world does not have has no version to release.
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn release_library_version(
    world: &impl crate::LibraryContext,
    library: &str,
) -> Result<LibraryRelease> {
    let version = match LibraryKind::of_file(library) {
        Some(kind) => {
            world
                .libraries()
                .versions
                .release_library_version(kind)
                .await?
        }
        None => None,
    };

    Ok(match version {
        Some(version) => LibraryRelease::Released(version),
        None => LibraryRelease::Failure(Failure::Admin(AdminFailure::UnknownLibrary(
            library.to_owned(),
        ))),
    })
}

/// Reports a weakness of the newest library file `library` (`FileHelperServer.ExploitReportServerRpc`).
///
/// # Errors
///
/// Returns errors from database reads and writes.
pub(super) async fn report_exploit(
    world: &impl crate::LibraryContext,
    library: &str,
    zone: &str,
    unsec_value: &str,
) -> Result<Outcome> {
    Ok(
        match world
            .libraries()
            .versions
            .report_weakness(library, zone, unsec_value)
            .await?
        {
            Ok(()) => Outcome::True,
            Err(failure) => Outcome::Failure(Failure::Admin(match failure {
                WeaknessReportFailure::Unreportable => AdminFailure::UnreportableLibrary,
                WeaknessReportFailure::UnknownLibrary(library) => {
                    AdminFailure::UnknownLibrary(library)
                }
                WeaknessReportFailure::UnknownZone => AdminFailure::UnknownReportedZone,
                WeaknessReportFailure::UnknownValue { value, zone } => {
                    AdminFailure::UnknownReportedValue { value, zone }
                }
                WeaknessReportFailure::Pending => AdminFailure::ReportPending,
            })),
        },
    )
}
