use greyhack_world_core::{Node, RouterId};

/// Target of a session opened by `metaxploit.net_use`.
#[derive(Debug, Clone, PartialEq)]
pub enum NetUseSession {
    /// Router kernel at `address` (`HelperValMetaLib.routerAddress`). Exploits recheck address
    /// reachability through firewalls.
    Router { router: RouterId, address: String },
    /// Library file backing a computer service.
    Service(Box<Node>),
}
