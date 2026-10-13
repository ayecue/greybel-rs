//! Intrinsic-specific Grey Hack file-failure text.

use crate::protocol::FileFailure;

/// Formats `failure` for `intrinsic` (`FileIntrinsics`, `ComputerIntrinsics`, `PlayerUtils.CrearArchivo`, `FileSystem.CopyFile`).
pub(in crate::game) fn file_text(intrinsic: &str, failure: &FileFailure) -> String {
    match failure {
        FileFailure::ComputerUnavailable => "unknown error".to_owned(),
        FileFailure::MissingFile(path) => match intrinsic {
            "delete" => format!("file not found: {path}"),
            "chmod" => format!("{path} not found."),
            "encrypt" | "decrypt" | "is_encrypted" => format!("{intrinsic}: File not found"),
            _ => format!("{path} not found"),
        },
        FileFailure::MissingFolder(folder) => match intrinsic {
            "symlink" => "Error: path folder not found".to_owned(),
            "touch" | "aireplay" | "buy_exploit" => {
                format!("Can't create file. {folder} does not exist.")
            }
            "create_folder" => format!("{folder} not found."),
            _ => format!("{folder} not found"),
        },
        FileFailure::NotAFolder(folder) => format!("{folder} it's not a folder"),
        FileFailure::InvalidPath => match intrinsic {
            "move" | "copy" => "Error: Invalid path",
            "symlink" => "Error: invalid path",
            _ => "Invalid path",
        }
        .to_owned(),
        FileFailure::EmptyPath => "Error: empty destination path".to_owned(),
        FileFailure::NoPermission => match intrinsic {
            "set_owner" | "set_group" => "Permission denied",
            _ => "permission denied",
        }
        .to_owned(),
        FileFailure::NoCreatePermission { folder, name } => {
            format!("Can't create file {folder}/{name}. Permission denied")
        }
        FileFailure::NoReadPermission(path) => format!("can't open {path}. Permission denied."),
        FileFailure::SystemFile => match intrinsic {
            "rename" => "permission denied. File protected",
            _ => "permission denied. File protected.",
        }
        .to_owned(),
        FileFailure::SystemFileTarget(name) => format!("permission denied. {name} is protected."),
        FileFailure::Exists(name) => match intrinsic {
            "create_folder" => format!("Error: {name} file exists"),
            _ => "The file already exists".to_owned(),
        },
        FileFailure::NameTaken => {
            "There is already a file with that name, please choose another one.".to_owned()
        }
        FileFailure::TooManyFiles => match intrinsic {
            "touch" | "symlink" | "aireplay" | "buy_exploit" => {
                "Can't create file. Reached maximum limit"
            }
            _ => "The copy can not be made. Reached maximum limit",
        }
        .to_owned(),
        FileFailure::TooManyFolders => "Can't create folder. Reached maximum limit".to_owned(),
        FileFailure::FolderFull => {
            "The copy can not be made. Reached maximum number of files in a folder".to_owned()
        }
        FileFailure::TooManySubfolders => {
            "The maximum number of subfolders has been exceeded".to_owned()
        }
        FileFailure::DiskFull => "There is not enough free space on the hard disk.".to_owned(),
        FileFailure::RootMove => "Error: The folder \"/\" cannot be moved.".to_owned(),
        FileFailure::RootDeletion => "unknown error :/".to_owned(),
        FileFailure::KindMismatch => "A file cannot overwrite a folder or vice versa".to_owned(),
        FileFailure::SymlinkToItself => {
            "Error: the symlink creation path cannot be the same as the file path".to_owned()
        }
        FileFailure::Binary(path) => format!("can't open {path}. Binary file."),
        FileFailure::WrongFormat => "Wrong format.".to_owned(),
        FileFailure::CannotAccess(path) => format!("permission denied: cannot access {path}"),
        FileFailure::CannotRead(path) => format!("permission denied: cannot read {path}"),
        FileFailure::CannotWrite(path) => format!("permission denied: cannot write {path}"),
        FileFailure::CannotReplace(path) => format!("permission denied: cannot replace {path}"),
        FileFailure::OverwriteSelf => "A file cannot overwrite itself".to_owned(),
        FileFailure::PartialCopy => {
            "Warning: one or more files or folders cannot be copied.".to_owned()
        }
        FileFailure::PartialDelete => {
            "Warning: one or more files or folders cannot be deleted.".to_owned()
        }
        FileFailure::DeletePermission(path) => {
            format!(
                "Deletion cancelled: permission denied: {path}. No files or folders were deleted."
            )
        }
        FileFailure::FolderReadPermission(path) => format!("Permission denied: {path}"),
    }
}
