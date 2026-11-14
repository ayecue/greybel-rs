/// A list of the missions the game authors (`MissionGen.missions`, one list for each `TypeMission` that is not
/// procedural).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionList {
    /// `MissionHidden`.
    Secret,
    /// `MissionKarma`.
    Grey,
    /// `MissionKarmaBlack`.
    Black,
    /// `MissionKarmaWhite`.
    White,
    /// `MissionZeroDay`.
    ZeroDay,
}

impl MissionList {
    pub const ALL: [Self; 5] = [
        Self::Secret,
        Self::Grey,
        Self::Black,
        Self::White,
        Self::ZeroDay,
    ];

    /// The list as the game's JSON text.
    #[must_use]
    pub fn text(self) -> &'static str {
        crate::bundle::text(match self {
            Self::Secret => "config/mission_hidden.json",
            Self::Grey => "config/mission_karma.json",
            Self::Black => "config/mission_karma_black.json",
            Self::White => "config/mission_karma_white.json",
            Self::ZeroDay => "config/mission_zero_day.json",
        })
        .trim_start_matches('\u{feff}')
    }
}
