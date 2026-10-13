use greybel_documents::ResourceProvider;
use greybel_documents_fs_provider::FsProvider;

#[test]
fn should_take_an_entry_as_a_file_uri_or_a_path() {
    let provider = FsProvider::default();
    let folder = std::env::temp_dir();
    let path = folder.join("main.src");
    let uri = FsProvider::uri(&path);
    let in_current_dir = FsProvider::uri(
        &std::env::current_dir()
            .expect("a directory")
            .join("main.src"),
    );
    let entries = [
        uri.to_string(),
        path.to_string_lossy().into_owned(),
        "main.src".to_owned(),
    ];
    let [from_uri, from_path, from_relative_path] = entries.map(|entry| provider.resolve(&entry));

    assert_eq!(from_uri, uri);
    assert_eq!(from_path, uri);
    assert_eq!(from_relative_path, in_current_dir);
}

#[cfg(windows)]
#[test]
fn should_take_a_drive_path_as_a_path() {
    let provider = FsProvider::default();
    let entries = ["C:\\project\\main.src", "c:/project/main.src"];
    let [backslashes, slashes] = entries.map(|entry| provider.resolve(entry).to_string());

    assert_eq!(backslashes, "file:///c%3A/project/main.src");
    assert_eq!(slashes, "file:///c%3A/project/main.src");
}
