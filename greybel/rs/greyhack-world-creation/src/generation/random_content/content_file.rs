use greyhack_behaviour::{FileData, FileKind};

/// A file put into a user's home.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ContentFile {
    pub user: String,
    pub name: String,
    pub data: FileData,
}

impl ContentFile {
    /// Builds the file `name` of type `kind` for `user` as `Archivo(name, content, user, typeFile, ...)`
    /// does: its size is that of a binary of the same name.
    pub(super) fn of_kind(user: &str, name: String, kind: FileKind, content: String) -> Self {
        Self {
            user: user.to_owned(),
            data: FileData {
                kind,
                content,
                size: FileData::binary_size(&name, false),
                ..FileData::default()
            },
            name,
        }
    }
}
