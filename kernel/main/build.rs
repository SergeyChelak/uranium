use std::env;
use std::path::PathBuf;

fn main() {
    let architecture = env::var("CARGO_CFG_TARGET_ARCH")
        .expect("cargo sets CARGO_CFG_TARGET_ARCH for build scripts");
    let manifest_directory = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR for build scripts"),
    );
    let script = manifest_directory
        .join("link")
        .join(format!("{architecture}.ld"));
    assert!(
        script.exists(),
        "no linker script for target architecture {architecture}"
    );
    println!("cargo::rustc-link-arg=-T{}", script.display());
    println!("cargo::rerun-if-changed={}", script.display());
}
