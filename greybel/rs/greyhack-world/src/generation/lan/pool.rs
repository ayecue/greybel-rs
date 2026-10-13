/// Server service pool (`PreCrearSubnet` `list`, the network service list).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pool {
    Public,
    Private,
}
