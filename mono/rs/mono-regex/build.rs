//! Links the Native AOT library for the `dotnet` feature.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MONO_REGEX_LIB_DIR");
    println!("cargo:rerun-if-env-changed=MONO_REGEX_DIST");
    if env::var_os("CARGO_FEATURE_DOTNET").is_none() {
        return;
    }

    let target = env::var("TARGET").expect("cargo sets TARGET");
    let Some(rid) = runtime_identifier(&target) else {
        panic!("the .NET regex engine isn't built for {target}");
    };
    let windows = rid.starts_with("win");
    let file = if windows {
        "MonoRegex.dll"
    } else if rid.starts_with("osx") {
        "libMonoRegex.dylib"
    } else {
        "libMonoRegex.so"
    };

    let directory = match env::var_os("MONO_REGEX_LIB_DIR") {
        Some(directory) => PathBuf::from(directory),
        None => match env::var_os("MONO_REGEX_DIST") {
            Some(dist) => PathBuf::from(dist).join(&rid),
            None => {
                panic!("set MONO_REGEX_LIB_DIR to the directory that contains the {rid} library")
            }
        },
    };
    let library = directory.join(file);

    println!("cargo:rerun-if-changed={}", library.display());
    assert!(
        library.is_file(),
        "{} is missing; build it with `make regex RIDS={rid}` in `mono/dotnet/` or set MONO_REGEX_LIB_DIR",
        library.display()
    );

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));

    std::fs::copy(&library, out.join(file)).expect("copies the library");
    println!("cargo:rustc-link-search=native={}", out.display());

    if !windows {
        println!("cargo:rustc-link-lib=dylib=MonoRegex");
    }

    println!("cargo:lib_dir={}", out.display());
}

/// The .NET runtime identifier `mono/dotnet/mono-regex/build.sh` builds for `target`.
fn runtime_identifier(target: &str) -> Option<String> {
    let arch = match target.split('-').next()? {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        _ => return None,
    };
    let os = if target.contains("apple-darwin") {
        "osx"
    } else if target.contains("linux-gnu") {
        "linux"
    } else if target.contains("windows") {
        "win"
    } else {
        return None;
    };

    Some(format!("{os}-{arch}"))
}
