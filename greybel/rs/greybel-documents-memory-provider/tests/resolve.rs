use greybel_documents::{ResourceProvider, Uri};
use greybel_documents_memory_provider::MemoryProvider;

#[test]
fn should_take_an_entry_as_a_memory_uri_or_a_path() {
    let provider = MemoryProvider::default();
    let entries = [
        "/project/main.src",
        "memory:///project/main.src",
        "C:/project/main.src",
    ];
    let [path, uri, drive_path] = entries.map(|entry| provider.resolve(entry));

    assert_eq!(path, Uri::memory("/project/main.src"));
    assert_eq!(uri, Uri::memory("/project/main.src"));
    assert_eq!(drive_path.scheme(), Uri::MEMORY);
}
