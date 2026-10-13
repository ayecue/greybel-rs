/// The game's procedural mission templates (`MissionGen.procMissionsConfig`, the `MissionProcedural` text), as JSON.
#[must_use]
pub fn procedural_missions() -> &'static str {
    crate::bundle::text("config/mission_procedural.json").trim_start_matches('\u{feff}')
}
