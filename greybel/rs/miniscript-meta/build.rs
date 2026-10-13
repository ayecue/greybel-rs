//! Bundles `data/` (asset-bundle) for the crate to embed: its JSON deflate-compressed, unpacked when the crate first
//! loads it.

use std::path::{Path, PathBuf};

use asset_bundle::BundleWriter;

fn main() {
    let data = Path::new("data");

    println!("cargo::rerun-if-changed=data");

    let mut files = Vec::new();

    collect(data, &mut files);
    files.sort();

    let bundle = files
        .iter()
        .fold(BundleWriter::new(), |bundle, path| {
            let name = path
                .strip_prefix(data)
                .expect("a file of data/")
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let contents =
                std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));

            bundle.with_file(name, contents)
        })
        .finish();
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));

    std::fs::write(out.join("data.bundle"), bundle).expect("the bundle is written");
}

fn collect(folder: &Path, files: &mut Vec<PathBuf>) {
    let entries =
        std::fs::read_dir(folder).unwrap_or_else(|error| panic!("{}: {error}", folder.display()));

    for entry in entries {
        let path = entry.expect("a data/ entry").path();

        if path.is_dir() {
            collect(&path, files);
        } else {
            files.push(path);
        }
    }
}
