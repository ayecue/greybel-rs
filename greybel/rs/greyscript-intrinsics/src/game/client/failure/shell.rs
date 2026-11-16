//! Grey Hack shell failure text for program launch and build operations.

use greyhack_behaviour::path;

use super::computer::{COMPUTER_UNAVAILABLE, process_text};
use crate::protocol::{ImportFailure, LaunchFailure, ShellFailure};

/// Formats `ShellIntrinsics.launch` receiving a game map without a shell helper.
pub(in crate::game) fn invalid_shell() -> &'static str {
    "Invalid shell"
}

/// Formats `ShellIntrinsics.connect_service` rejecting a forbidden character in its password argument.
pub(in crate::game) fn invalid_password_text(token: &str) -> String {
    format!("Error: invalid character '{token}' in password argument.")
}

/// Formats `ShellIntrinsics.launch` failures printed by the terminal.
pub(in crate::game) fn launch_text(failure: &LaunchFailure) -> String {
    match failure {
        LaunchFailure::CommandNotFound(path) => format!("{path}: command not found"),
        LaunchFailure::NotExecutable(name) => format!("{name} is not an executable file."),
        LaunchFailure::NoPermission => "Can't launch program. Permission denied.".to_owned(),
        LaunchFailure::InvalidParameter(token) => {
            format!("Error: invalid character '{token}' in program parameters.")
        }
        LaunchFailure::TooManyStackCalls => "Program interrupted. Too many stack calls.".to_owned(),
        LaunchFailure::Process(failure) => process_text(*failure),
    }
}

/// Formats `ShellIntrinsics.build` and `ShellIntrinsics.connect_service` failures.
pub(in crate::game) fn shell_text(failure: &ShellFailure) -> String {
    match failure {
        ShellFailure::ComputerUnavailable => COMPUTER_UNAVAILABLE.to_owned(),
        ShellFailure::InvalidPasswordCharacter(token) => invalid_password_text(token),
        ShellFailure::MissingSource(path) => format!("Can't find {path}"),
        ShellFailure::MissingFolder(path) => format!("Can't find {}", path::parent(path)),
        ShellFailure::UnreadableSource(path) => {
            format!("Can't access to {path}. Permission denied.")
        }
        ShellFailure::BinarySource(name) => format!("Can't build {name}. Binary file"),
        ShellFailure::UnwritableFolder(folder) => {
            format!("Can't create binary in {folder}. Permission denied.")
        }
        ShellFailure::InvalidExtension(name) => format!("Can't build {name}. Invalid extension."),
        ShellFailure::EmptySource => "Can't compile. Source code is empty".to_owned(),
        // `PlayerUtils.ImportCode` failures are compiler errors; `build` returns their descriptions.
        ShellFailure::Import(failure) => format!("Compiler Error: {}", import_text(failure)),
    }
}

/// Formats `PlayerUtils.ImportCode` failures.
fn import_text(failure: &ImportFailure) -> String {
    match failure {
        ImportFailure::RelativePath(path) => {
            format!("import_code: {path} it is not a valid absolute path")
        }
        ImportFailure::MissingFile(path) => format!("import_code: File path: {path} not found."),
        ImportFailure::ImportedTwice(path) => format!("{path} already imported."),
        ImportFailure::TooLarge => {
            "import_code: The 4mb limit for code inclusion has been exceeded".to_owned()
        }
        ImportFailure::BinaryFile(path) => format!("import_code: {path} is a binary file"),
        ImportFailure::NoImportPermission(path) => {
            format!("import_code: {path} does not allow to be imported")
        }
        ImportFailure::Unreadable(path) => format!("can't open {path}. Permission denied."),
    }
}
