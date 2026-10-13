//! NPC jobs (`JobRole`).

/// NPC job (`JobRole`) identified by Grey Hack numeric code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JobRole {
    #[default]
    None = 0,
    TechnicalSupport = 1,
    SoftwareDeveloper = 2,
    DataAnalyst = 3,
    GraphicDesigner = 4,
    ProjectManager = 5,
    Sales = 6,
    Admin = 7,
    Sergeant = 8,
    Inspector = 9,
    Commissioner = 10,
}

impl JobRole {
    /// All roles in Grey Hack order.
    pub const ALL: [Self; 11] = [
        Self::None,
        Self::TechnicalSupport,
        Self::SoftwareDeveloper,
        Self::DataAnalyst,
        Self::GraphicDesigner,
        Self::ProjectManager,
        Self::Sales,
        Self::Admin,
        Self::Sergeant,
        Self::Inspector,
        Self::Commissioner,
    ];
    /// Police roles whose workstation accounts can access traffic cameras
    /// (`NetworkLan.SetTrafficCredentials`).
    pub const POLICE: [Self; 3] = [Self::Commissioner, Self::Inspector, Self::Sergeant];

    /// Roles available to network NPCs (`NetworkLan.GlobalLan.availableRoles`): police roles for
    /// police networks, all other roles otherwise.
    #[must_use]
    pub const fn available_for(police: bool) -> &'static [Self] {
        if police {
            &[
                Self::TechnicalSupport,
                Self::SoftwareDeveloper,
                Self::DataAnalyst,
                Self::Commissioner,
                Self::Inspector,
                Self::Sergeant,
            ]
        } else {
            &[
                Self::TechnicalSupport,
                Self::SoftwareDeveloper,
                Self::DataAnalyst,
                Self::GraphicDesigner,
                Self::ProjectManager,
                Self::Sales,
            ]
        }
    }

    /// Returns the role numbered `code` in Grey Hack order.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.get(usize::from(code)).copied()
    }

    /// Returns Grey Hack's numeric role code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Returns the role name used by Grey Hack (`JobRole.ToString`).
    #[must_use]
    pub const fn game_name(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::TechnicalSupport => "Technical_Support",
            Self::SoftwareDeveloper => "Software_Developer",
            Self::DataAnalyst => "Data_Analyst",
            Self::GraphicDesigner => "Graphic_Designer",
            Self::ProjectManager => "Project_Manager",
            Self::Sales => "Sales",
            Self::Admin => "Admin",
            Self::Sergeant => "Sergeant",
            Self::Inspector => "Inspector",
            Self::Commissioner => "Commissioner",
        }
    }
}
