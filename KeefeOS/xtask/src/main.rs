use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

const KERNEL_TARGET: &str = "x86_64-unknown-none";
const BOOTLOADER_TARGET: &str = "x86_64-unknown-uefi";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xtask <command> [options]");
        eprintln!("Commands:");
        eprintln!("  build [--release]  - Build kernel and bootloader, create disk image, launch QEMU");
        eprintln!("  run [--release]    - Alias for build");
        eprintln!("  image [--release]  - Create disk image only");
        exit(1);
    }

    let command = &args[1];
    let release = args.iter().any(|arg| arg == "--release");

    match command.as_str() {
        "build" | "run" => {
            build_all(release);
            create_image(release);
            run_qemu(release);
        }
        "image" => create_image(release),
        _ => {
            eprintln!("Unknown command: {}", command);
            exit(1);
        }
    }
}

fn build_all(release: bool) {
    println!("Building kernel for {KERNEL_TARGET}...");

    let mut kernel = Command::new("cargo");
    kernel.args([
        "build",
        "--package",
        "keefeos-kernel",
        "--target",
        KERNEL_TARGET,
    ]);
    if release {
        kernel.arg("--release");
    }

    let status = kernel.status().unwrap_or_else(|error| {
        eprintln!("Failed to start the kernel build: {error}");
        exit(1);
    });
    if !status.success() {
        eprintln!("Kernel build failed; refusing to package a disk image.");
        exit(status.code().unwrap_or(1));
    }

    println!("Building UEFI bootloader for {BOOTLOADER_TARGET}...");

    let mut bootloader = Command::new("cargo");
    bootloader.args([
        "build",
        "--package",
        "keefeos-bootloader",
        "--target",
        BOOTLOADER_TARGET,
    ]);
    if release {
        bootloader.arg("--release");
    }

    let status = bootloader.status().unwrap_or_else(|error| {
        eprintln!("Failed to start the UEFI bootloader build: {error}");
        exit(1);
    });
    if !status.success() {
        eprintln!("UEFI bootloader build failed; refusing to package a disk image.");
        exit(status.code().unwrap_or(1));
    }

    println!("Both build steps completed successfully.");
}

fn get_profile(release: bool) -> &'static str {
    if release {
        "release"
    } else {
        "debug"
    }
}

fn target_directory(workspace_root: &Path) -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR") {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                workspace_root.join(path)
            }
        }
        None => workspace_root.join("target"),
    }
}

fn create_image(release: bool) {
    println!("Creating FAT disk image...");

    let profile = get_profile(release);
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the KeefeOS workspace")
        .to_path_buf();
    let target_dir = target_directory(&workspace_root);

    let kernel_bin = target_dir
        .join(KERNEL_TARGET)
        .join(profile)
        .join("keefeos-kernel");
    let bootloader_efi = target_dir
        .join(BOOTLOADER_TARGET)
        .join(profile)
        .join("keefeos-bootloader.efi");

    if !kernel_bin.is_file() {
        eprintln!("Kernel binary not found: {}", kernel_bin.display());
        eprintln!("Build it first with cargo build --package keefeos-kernel --target {KERNEL_TARGET}.");
        exit(1);
    }
    if !bootloader_efi.is_file() {
        eprintln!("UEFI bootloader not found: {}", bootloader_efi.display());
        eprintln!("Build it first with cargo build --package keefeos-bootloader --target {BOOTLOADER_TARGET}.");
        exit(1);
    }

    let image_path = target_dir.join("keefeos.img");
    let image_size = 64 * 1024 * 1024;

    let image_file = File::create(&image_path).unwrap_or_else(|error| {
        eprintln!("Failed to create {}: {error}", image_path.display());
        exit(1);
    });
    image_file.set_len(image_size as u64).unwrap_or_else(|error| {
        eprintln!("Failed to size {}: {error}", image_path.display());
        exit(1);
    });
    drop(image_file);

    format_fat_image(&image_path, &kernel_bin, &bootloader_efi).unwrap_or_else(|error| {
        eprintln!("Failed to format disk image {}: {error}", image_path.display());
        exit(1);
    });

    println!("Disk image created: {}", image_path.display());
}

fn format_fat_image(
    image_path: &Path,
    kernel_bin: &Path,
    bootloader_efi: &Path,
) -> std::io::Result<()> {
    use fatfs::{FileSystem, FormatVolumeOptions, FsOptions};

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(image_path)?;

    let format_options = FormatVolumeOptions::new().volume_label(*b"KEEFEOS    ");
    fatfs::format_volume(&file, format_options)?;

    let fs = FileSystem::new(&file, FsOptions::new())?;
    let root_dir = fs.root_dir();
    let efi_dir = root_dir.create_dir("EFI")?;
    let boot_dir = efi_dir.create_dir("BOOT")?;

    let mut bootloader_data = Vec::new();
    File::open(bootloader_efi)?.read_to_end(&mut bootloader_data)?;
    let mut bootloader_file = boot_dir.create_file("BOOTX64.EFI")?;
    bootloader_file.write_all(&bootloader_data)?;

    let mut kernel_data = Vec::new();
    File::open(kernel_bin)?.read_to_end(&mut kernel_data)?;
    let mut kernel_file = root_dir.create_file("KERNEL.ELF")?;
    kernel_file.write_all(&kernel_data)?;

    Ok(())
}

fn run_qemu(_release: bool) {
    println!("Launching QEMU...");

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be inside the KeefeOS workspace")
        .to_path_buf();
    let image_path = target_directory(&workspace_root).join("keefeos.img");

    if !image_path.is_file() {
        eprintln!("Disk image not found: {}", image_path.display());
        eprintln!("Run the image command after building both target artifacts.");
        exit(1);
    }

    let ovmf_code_paths = [
        "/usr/share/OVMF/OVMF_CODE.fd",
        "/usr/share/ovmf/OVMF_CODE.fd",
        "/usr/share/qemu/OVMF_CODE.fd",
    ];
    let ovmf_vars_paths = [
        "/usr/share/OVMF/OVMF_VARS.fd",
        "/usr/share/ovmf/OVMF_VARS.fd",
        "/usr/share/qemu/OVMF_VARS.fd",
    ];

    let ovmf_code = ovmf_code_paths
        .iter()
        .find(|path| Path::new(path).is_file())
        .copied();
    let ovmf_vars = ovmf_vars_paths
        .iter()
        .find(|path| Path::new(path).is_file())
        .copied();

    let (Some(ovmf_code), Some(ovmf_vars)) = (ovmf_code, ovmf_vars) else {
        eprintln!("OVMF firmware not found. Install QEMU and an OVMF package, then retry.");
        exit(1);
    };

    let status = Command::new("qemu-system-x86_64")
        .args([
            "-drive",
            &format!("if=pflash,format=raw,readonly=on,file={ovmf_code}"),
            "-drive",
            &format!("if=pflash,format=raw,file={ovmf_vars}"),
            "-drive",
            &format!("format=raw,file={}", image_path.display()),
            "-serial",
            "stdio",
            "-m",
            "256M",
            "-cpu",
            "qemu64",
        ])
        .status()
        .unwrap_or_else(|error| {
            eprintln!("Failed to launch qemu-system-x86_64: {error}");
            exit(1);
        });

    if !status.success() {
        eprintln!("QEMU exited unsuccessfully with status {status}.");
        exit(status.code().unwrap_or(1));
    }
}
