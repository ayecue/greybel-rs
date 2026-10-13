/// An account's name and groups, as the world's users carry them.
pub struct Account {
    pub name: String,
    pub groups: Vec<String>,
}

pub fn user(name: &str, groups: &[&str]) -> Account {
    Account {
        name: name.to_owned(),
        groups: groups.iter().map(|group| (*group).to_owned()).collect(),
    }
}
