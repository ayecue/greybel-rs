//! Files a computer creates for its accounts: `/etc/passwd` and home folders.

use greyhack_behaviour::{FileData, PASSWD_NAME, PermissionSet, ROOT_USER, passwd_content};

use crate::model::{FileSystemDraft, NodeDraft, NodeId, User};

/// Returns the `/etc/passwd` lines of `users`, one `name:md5` line per user.
pub(crate) fn passwd_text(users: &[User]) -> String {
    passwd_content(
        users
            .iter()
            .map(|user| (user.name.as_str(), user.password_hash.as_str())),
    )
}

/// Returns whether an existing `/etc/passwd` with `data` is updated in place rather than replaced
/// (`Computer.GeneratePasswdFile`): replacement happens when forced or when the file is binary.
pub(crate) fn passwd_is_reusable(data: Option<&FileData>, recreate: bool) -> bool {
    !recreate && !data.is_some_and(FileData::is_binary)
}

/// Returns the `/etc` folder created when a computer lacks one (`Computer.GeneratePasswdFile`).
pub(crate) fn etc_folder() -> NodeDraft {
    NodeDraft::folder("etc", ROOT_USER, PermissionSet::default_for(true))
}

/// Returns the empty `/etc/passwd` file created when a computer lacks one
/// (`Computer.GeneratePasswdFile`).
pub(crate) fn passwd_file_draft() -> NodeDraft {
    NodeDraft::file(
        PASSWD_NAME,
        ROOT_USER,
        PermissionSet::default_for(false),
        FileData::text(""),
    )
}

/// Returns the home folders of `owner` ([`greyhack_behaviour::home_folders`]) with `folder_permissions` on the folders
/// the user shares with the group.
pub(crate) fn home_folders(owner: &str, folder_permissions: PermissionSet) -> Vec<NodeDraft> {
    greyhack_behaviour::home_folders(folder_permissions)
        .into_iter()
        .map(|(name, permissions)| {
            let mut folder = NodeDraft::folder(name, owner, permissions);

            folder.timestamp = greyhack_behaviour::parse_date(greyhack_behaviour::START_DATE);

            folder
        })
        .collect()
}

pub(crate) fn add_home_folders(
    fs: &mut FileSystemDraft,
    folder: NodeId,
    owner: &str,
    folder_permissions: PermissionSet,
) {
    for node in home_folders(owner, folder_permissions) {
        fs.add(folder, node);
    }
}
