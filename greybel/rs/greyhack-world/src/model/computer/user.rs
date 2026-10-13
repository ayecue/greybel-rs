use greyhack_behaviour::{GUEST_USER, ROOT_USER, hash_password};

use crate::model::PersonId;

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
}
