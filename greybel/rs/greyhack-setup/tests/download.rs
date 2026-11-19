use greyhack_setup::{Downloader, SetupError};

use crate::support::TemporaryFolder;
use crate::support::download::FileServer;

#[test]
fn should_reuse_a_cached_file_of_the_expected_sha256() {
    let cache = TemporaryFolder::new();
    let file = cache.write("BepInEx.zip", "archive");
    let sha256 = Downloader::sha256_of(&file).expect("its checksum");

    let path = Downloader::new(cache.path().to_path_buf())
        .fetch("http://127.0.0.1:9/never", "", "BepInEx.zip", &sha256)
        .expect("the cached file");

    assert_eq!(path, file);
}

#[test]
fn should_report_a_failed_download() {
    let cache = TemporaryFolder::new();

    let error = Downloader::new(cache.path().to_path_buf())
        .fetch(
            "http://127.0.0.1:9/never",
            "",
            "BepInEx.zip",
            &"0".repeat(64),
        )
        .expect_err("nothing listens");

    assert!(
        error
            .to_string()
            .starts_with("Couldn't download http://127.0.0.1:9/never")
    );
    assert!(!cache.join("BepInEx.zip.part").exists());
}

#[test]
fn should_download_a_file_with_the_expected_sha256() {
    let cache = TemporaryFolder::new();
    let address = FileServer::serve("archive");
    let reference_file = cache.write("reference", "archive");
    let sha256 = Downloader::sha256_of(&reference_file).expect("its checksum");

    let path = Downloader::new(cache.path().to_path_buf())
        .fetch(&address, "v1", "file.zip", &sha256)
        .expect("downloaded");

    assert_eq!(path, cache.join("v1/file.zip"));
    assert_eq!(std::fs::read_to_string(path).expect("the file"), "archive");
}

#[test]
fn should_reject_a_download_with_another_sha256() {
    let cache = TemporaryFolder::new();
    let address = FileServer::serve("changed");

    let error = Downloader::new(cache.path().to_path_buf())
        .fetch(&address, "", "file.zip", &"0".repeat(64))
        .expect_err("another checksum");

    assert!(matches!(error, SetupError::ChecksumMismatch { .. }));
    assert!(!cache.join("file.zip").exists());
    assert!(!cache.join("file.zip.part").exists());
}
