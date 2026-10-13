//! Helpers for asserting member search results.

use super::reader::MemberPath;

pub fn path(owner: &str, name: &str) -> MemberPath {
    MemberPath {
        owner: owner.into(),
        name: name.into(),
    }
}
