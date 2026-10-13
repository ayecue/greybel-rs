//! People (`Persona`) with names, mail accounts, and bank accounts used by world accounts.

use greyhack_behaviour::enum_def;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonId(pub(crate) u32);

enum_def! {
    pub enum Gender {
        Female = 0,
        Male = 1,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub surname: String,
    pub gender: Gender,
    pub mail_address: Option<String>,
    pub bank_number: Option<String>,
}
