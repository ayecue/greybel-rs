use greyhack_behaviour::{
    FileData, GUEST_USER, PASSWD_NAME, PermissionSet, ROOT_USER, hash_password, passwd_content,
};

use crate::model::{FileSystemDraft, NodeDraft, NodeId, PersonId};

/// Computer account (`Computer.User`). Each user has a same-named group. Plaintext passwords are
/// stored once by hash in `passwords` and loaded with the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub name: String,
    /// Password MD5 stored in `/etc/passwd`.
    pub password_hash: String,
    pub password: String,
    pub groups: Vec<String>,
    pub deletable: bool,
    /// Person owning the account.
    pub person: Option<PersonId>,
}

impl User {
    #[must_use]
    pub fn new(name: &str, password: &str) -> Self {
        Self {
            name: name.to_owned(),
            password_hash: hash_password(password),
            password: password.to_owned(),
            groups: vec![name.to_owned()],
            deletable: false,
            person: None,
        }
    }

    /// Returns the implicit `guest` account available on every computer but omitted from its user
    /// list.
    #[must_use]
    pub fn guest() -> Self {
        Self {
            password_hash: String::new(),
            ..Self::new(GUEST_USER, "")
        }
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.name == ROOT_USER
    }

    pub fn add_group(&mut self, group: &str) {
        if !self.groups.iter().any(|group_name| group_name == group) {
            self.groups.push(group.to_owned());
        }
    }

    pub fn set_password(&mut self, password: &str) {
        password.clone_into(&mut self.password);
        self.password_hash = hash_password(password);
    }

    /// Returns the `/etc/passwd` lines of `users`, one `name:md5` line per user.
    #[must_use]
    pub fn passwd_text(users: &[User]) -> String {
        passwd_content(
            users
                .iter()
                .map(|user| (user.name.as_str(), user.password_hash.as_str())),
        )
    }

    /// Returns whether an existing `/etc/passwd` is updated in place rather than replaced
    /// (`Computer.GeneratePasswdFile`).
    #[must_use]
    pub fn passwd_is_reusable(data: Option<&FileData>, recreate: bool) -> bool {
        !recreate && !data.is_some_and(FileData::is_binary)
    }

    /// Returns the `/etc` folder created when a computer lacks one.
    #[must_use]
    pub fn etc_folder<L>() -> NodeDraft<L> {
        NodeDraft::folder("etc", ROOT_USER, PermissionSet::default_for(true))
    }

    /// Returns the empty `/etc/passwd` file created when a computer lacks one.
    #[must_use]
    pub fn passwd_file_draft<L>() -> NodeDraft<L> {
        NodeDraft::file(
            PASSWD_NAME,
            ROOT_USER,
            PermissionSet::default_for(false),
            FileData::text(""),
        )
    }

    /// Returns the home folders of `owner` with `folder_permissions` on shared group folders.
    #[must_use]
    pub fn home_folders<L>(owner: &str, folder_permissions: PermissionSet) -> Vec<NodeDraft<L>> {
        greyhack_behaviour::home_folders(folder_permissions)
            .into_iter()
            .map(|(name, permissions)| {
                let mut folder = NodeDraft::folder(name, owner, permissions);

                folder.timestamp = greyhack_behaviour::parse_date(greyhack_behaviour::START_DATE);

                folder
            })
            .collect()
    }

    /// Adds the home folders of `owner` under `folder`.
    pub fn add_home_folders<L>(
        fs: &mut FileSystemDraft<L>,
        folder: NodeId,
        owner: &str,
        folder_permissions: PermissionSet,
    ) {
        for node in Self::home_folders(owner, folder_permissions) {
            fs.add(folder, node);
        }
    }
}
