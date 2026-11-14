use greyhack_world_assets::NameList;

use crate::Rng;

enum_def! {
    /// A person's gender (`Persona.Genero`).
    pub enum Gender {
        Female = 0,
        Male = 1,
    }
}

impl Gender {
    /// Draws a gender with an even chance of each.
    pub fn pick(rng: &mut Rng) -> Self {
        if rng.range(0, 1) == 0 {
            Self::Female
        } else {
            Self::Male
        }
    }

    /// Selects a first name from the gender list, excluding the trailing empty entry.
    pub fn first_name(self, rng: &mut Rng) -> String {
        self.first_name_with_index(|length| rng.index(length))
    }

    /// Selects a first name (`WordGenerator.GetNombre`) at the index `index` draws below the
    /// number of names.
    pub fn first_name_with_index(self, index: impl FnOnce(usize) -> usize) -> String {
        let list = match self {
            Self::Female => NameList::FemaleNames,
            Self::Male => NameList::MaleNames,
        };
        let names = list.entries();
        let names = names.strip_suffix(&[""]).unwrap_or(names);
        let name = names[index(names.len())];
        let mut chars = name.chars();

        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().chain(chars).collect()
        })
    }
}
