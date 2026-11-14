use super::ZeroDayObject;
use crate::Weakness;

#[derive(Debug, Clone, PartialEq)]
pub struct ZeroDay {
    pub address: String,
    pub objects: Vec<ZeroDayObject>,
    pub vulnerability: Weakness,
}
