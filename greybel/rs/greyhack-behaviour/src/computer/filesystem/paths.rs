//! Well-known paths of files and folders, and the folders of a new home.

use crate::computer::permissions::PermissionSet;
use crate::path;

/// Directory containing the account database.
pub const PASSWD_FOLDER: &str = "/etc";
/// Folders of every home folder besides [`TRASH_FOLDER`] (`FileSystem.GenerateHomeFolders`).
const HOME_FOLDERS: [&str; 3] = ["Desktop", "Downloads", "Config"];
/// Private trash folder of every home folder (`FileSystem.GenerateHomeFolders`).
const TRASH_FOLDER: &str = ".Trash";
/// Filename of the account database.
pub const PASSWD_NAME: &str = "passwd";
/// Full path of the account database.
pub const PASSWD_PATH: &str = "/etc/passwd";
/// Path of the terminal program run by `start_terminal` (`Computer.UseCustomBash`).
pub const BASH_PATH: &str = "/bin/bash";
/// Folder the programs of a hosted website are offered for download from
/// (`Computer.ConfiguringServices`).
pub const DOWNLOADS_PATH: &str = "/Public/htdocs/downloads";
/// The page of a hosted website (`Computer.ConfiguringServices`).
pub const WEBSITE_PATH: &str = "/Public/htdocs/website.html";

/// Returns the path of the file with the mail account of `user` (`Persona.GeneraMail`).
#[must_use]
pub fn mail_config_path(user: &str) -> String {
    format!("{}/Config/Mail.txt", path::home(user))
}

/// Returns the path of the file with the bank account of `user` (`Persona.SetBanco`).
#[must_use]
pub fn bank_config_path(user: &str) -> String {
    format!("{}/Config/Bank.txt", path::home(user))
}

/// Returns the folders and permissions of a new home folder: [`HOME_FOLDERS`] with `folder_permissions`
/// permissions, then the private [`TRASH_FOLDER`] (`FileSystem.GenerateHomeFolders`). Regular
/// users use [`PermissionSet::GROUP_FOLDER`] and `guest` [`PermissionSet::OPEN_FOLDER`].
#[must_use]
pub fn home_folders(folder_permissions: PermissionSet) -> [(&'static str, PermissionSet); 4] {
    let [desktop, downloads, config] = HOME_FOLDERS;

    [
        (desktop, folder_permissions),
        (downloads, folder_permissions),
        (config, folder_permissions),
        (TRASH_FOLDER, PermissionSet::PRIVATE_FOLDER),
    ]
}
