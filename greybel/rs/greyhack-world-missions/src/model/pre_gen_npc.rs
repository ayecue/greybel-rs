/// `MissionConfig.PreGenNpc`: an NPC a mission needs on its computers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PreGenNpc {
    pub name: String,
    pub surname: String,
    pub is_victim: bool,
    pub is_guilty: bool,
    pub is_forced_pass: bool,
    pub forced_pass: Option<String>,
}

impl PreGenNpc {
    /// `PreGenNpc.GetCompleteName`.
    pub fn complete_name(&self) -> String {
        format!("{} {}", self.name, self.surname)
            .trim_start()
            .to_owned()
    }
}
