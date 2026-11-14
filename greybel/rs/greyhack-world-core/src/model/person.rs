//! People (`Persona`) and their base identity. Mail and bank links belong to their domains.

pub use greyhack_behaviour::Gender;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub name: String,
    pub surname: String,
    pub gender: Gender,
}
