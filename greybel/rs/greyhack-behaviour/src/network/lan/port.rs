/// Router port-table entry (`Puerto`) mapping a public port number to a LAN address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanPort {
    pub number: u16,
    pub lan_ip: String,
}
