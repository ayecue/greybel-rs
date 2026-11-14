//! Mails an NPC account receives the first time it is used (`Persona.GenerateMailsContents`).

pub(crate) mod administrator;
pub(crate) mod colleague;
pub(crate) mod delivery;
pub(crate) mod draw;
pub(crate) mod owner;

pub(crate) use administrator::Administrator;
pub(crate) use colleague::Colleague;
pub(crate) use delivery::Delivery;
pub(crate) use draw::contents;
pub(crate) use owner::Owner;

#[cfg(test)]
#[path = "tests/mod.rs"]
pub(crate) mod tests;
