//! Mails an NPC account receives the first time it is used (`Persona.GenerateMailsContents`).

mod administrator;
mod colleague;
mod delivery;
mod draw;
mod owner;

pub(crate) use administrator::Administrator;
pub(crate) use colleague::Colleague;
pub(crate) use delivery::Delivery;
pub(crate) use draw::contents;
pub(crate) use owner::Owner;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
