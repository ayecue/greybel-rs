use greyhack_behaviour::{hash_password, verify_password};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugAccount {
    pub user: String,
    pub password_hash: String,
    pub failed_logins: u32,
    pub unit_test_failed: bool,
    pub last_network_ip: Option<String>,
}

impl DebugAccount {
    #[must_use]
    pub fn new(user: &str, password: &str) -> Self {
        Self {
            user: user.to_owned(),
            password_hash: hash_password(password),
            failed_logins: 0,
            unit_test_failed: false,
            last_network_ip: None,
        }
    }

    #[must_use]
    pub fn check_password(&self, password: &str) -> bool {
        verify_password(&self.password_hash, password)
    }
}
