use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"),
    );
    let workspace_root = manifest_dir
        .parent()
        .expect("bootloader manifest must be inside the KeefeOS workspace");

    let target_root = match env::var_os("CARGO_TARGET_DIR") {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        }
        None => workspace_root.join("target"),
    };

    let profile = env::var("PROFILE").expect("PROFILE is set by Cargo");
    let kernel_path = target_root
        .join("x86_64-unknown-none")
        .join(&profile)
        .join("keefeos-kernel");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let embedded_kernel = out_dir.join("keefeos-kernel");

    println!("cargo:rerun-if-env-changed=CARGO_TARGET_DIR");
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-changed={}", kernel_path.display());

    if !kernel_path.is_file() {
        let release_flag = if profile == "release" { " --release" } else { "" };
        panic!(
            "kernel artifact not found at {}. Build the matching kernel profile first with: cargo build --package keefeos-kernel --target x86_64-unknown-none{}",
            kernel_path.display(),
            release_flag
        );
    }

    fs::copy(&kernel_path, &embedded_kernel).unwrap_or_else(|error| {
        panic!(
            "failed to copy kernel artifact from {} to {}: {}",
            kernel_path.display(),
            embedded_kernel.display(),
            error
        )
    });
}
