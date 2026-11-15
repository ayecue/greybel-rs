use greyhack_behaviour::NetworkAdmin;

/// The network administrator (`NetworkLan.GetAdminNpcInfo`).
#[derive(Debug, Clone)]
pub(crate) struct Administrator {
    pub surname: String,
    pub stats: NetworkAdmin,
}
