//! Programs and files of the saved-world tests.

pub const WRITE: &str = "get_shell.host_computer.touch(home_dir, \"kept.txt\")";
pub const READ: &str = "print get_shell.host_computer.File(home_dir + \"/kept.txt\") != null";

/// Names of the world databases in `folder`.
pub fn databases(folder: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .expect("the folder is read")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".db"))
        .collect();

    names.sort();

    names
}

/// Changes the recorded layout of every world listed in `folder`'s `worlds.json` with `change`.
pub fn change_formats(folder: &std::path::Path, change: impl Fn(&mut serde_json::Value)) {
    let path = folder.join("worlds.json");
    let mut index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the index is read"))
            .expect("the index parses");

    for entry in index["worlds"]
        .as_object_mut()
        .expect("worlds")
        .values_mut()
    {
        change(&mut entry["format"]);
    }

    std::fs::write(&path, index.to_string()).expect("the index is written");
}

/// Names in `folder` ending with `suffix`.
pub fn files_with_suffix(folder: &std::path::Path, suffix: &str) -> Vec<String> {
    std::fs::read_dir(folder)
        .expect("the folder is read")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(suffix))
        .collect()
}
