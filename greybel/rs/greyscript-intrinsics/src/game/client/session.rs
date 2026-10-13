use super::object::Access;

/// Script execution identity and filesystem paths (`GreyInterpreter`'s `activeUser`, `currentPath`, and related state).
#[derive(Debug, Clone)]
pub struct Session {
    /// Server shell handle for the script's computer and user.
    pub(crate) shell: Access,
    pub user: String,
    pub current_path: String,
    /// Original player-launch path.
    pub launch_path: String,
    /// Executing program's path.
    pub program_path: String,
}
