pub mod requirements;
pub mod runner;
pub mod values;

pub const SCENARIOS: &str = "scenarios";

/// Loads all shared scenarios as `(file_stem, JSON value)` pairs.
pub fn scenario_files() -> Vec<(String, serde_json::Value)> {
    let folder = std::path::Path::new(env!("DUPLEX")).join(SCENARIOS);
    let mut paths: Vec<_> = std::fs::read_dir(&folder)
        .expect("the scenario directory is readable")
        .map(|entry| {
            entry
                .expect("each scenario directory entry is readable")
                .path()
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();

    paths.sort();
    assert!(!paths.is_empty(), "no scenarios in {}", folder.display());

    paths
        .iter()
        .map(|path| {
            let name = path
                .file_stem()
                .expect("a scenario file name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(path).expect("the scenario file is readable");
            let scenario =
                serde_json::from_str(&text).expect("the scenario file contains valid json");

            (name, scenario)
        })
        .collect()
}
