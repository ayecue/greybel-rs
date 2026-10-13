use indexmap::IndexMap;
use serde::Serialize;
use serde::de::{Deserialize, Deserializer};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Description {
    description: String,
    example: Option<Vec<String>>,
}

impl Description {
    #[must_use]
    pub fn text(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub fn example(&self) -> Option<&[String]> {
        self.example.as_deref()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DescriptionTable {
    type_description: Option<Description>,
    members: IndexMap<String, Description>,
}

impl DescriptionTable {
    const TYPE_KEY: &'static str = "$meta";

    #[must_use]
    pub fn type_description(&self) -> Option<&Description> {
        self.type_description.as_ref()
    }

    #[must_use]
    pub fn members(&self) -> &IndexMap<String, Description> {
        &self.members
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if other.type_description.is_some() {
            self.type_description = other.type_description;
        }

        self.members.extend(other.members);
    }
}

impl<'de> Deserialize<'de> for DescriptionTable {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut members = IndexMap::<String, Description>::deserialize(deserializer)?;
        let type_description = members.shift_remove(Self::TYPE_KEY);

        Ok(Self {
            type_description,
            members,
        })
    }
}
