/// Why the game cannot build a mission (it throws).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BuildError {
    /// No archetype of the source has a heart option (`ProcMissionBuilder.SelectArchetype`).
    NoTemplates,
    /// An archetype has no starter, or no closure of the starter's group (`PickStarter`, `PickClosure`).
    MissingTemplate,
    /// A template uses a next-node tag without a next beat on the same network.
    MissingBeat,
    /// No candidate fits the available networks (`BuildMission`).
    NetworkLimit,
}
