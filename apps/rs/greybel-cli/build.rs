//! Sets `GREYBEL_TARGET` to the Rust target this program is built for, which names its archive in a release.

fn main() {
    let target = std::env::var("TARGET").expect("Cargo sets TARGET");

    println!("cargo::rustc-env=GREYBEL_TARGET={target}");
}
