use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

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
        "image" => {
            create_image(release);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            exit(1);
        }
    }
}

fn build_all(release: bool) {
    println!("Building kernel and bootloader...");

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--package", "keefeos-kernel"]);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status().expect("Failed to build kernel");
    if !status.success() {
        eprintln!("Kernel build failed");
        exit(1);
    }

    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--package", "keefeos-bootloader"]);
    if release {
        cmd.arg("--release");
    }
    // Bootloader needs UEFI target
    cmd.env("CARGO_TARGET_X86_64_UNKNOWN_UEFI_RUNNER", "");
    let status = cmd.status().expect("Failed to build bootloader");
    if !status.success() {
        eprintln!("Bootloader build failed");
        exit(1);
    }

    println!("Build completed successfully");
}

fn get_profile(release: bool) -> &'static str {
    if release { "release" } else { "debug" }
}

fn create_image(release: bool) {
    println!("Creating FAT disk image...");

    let profile = get_profile(release);
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let target_dir = workspace_root.join("target");
    
    // Paths to built binaries
    let kernel_bin = target_dir.join("x86_64-unknown-none").join(profile).join("keefeos-kernel");
    let bootloader_efi = target_dir.join("x86_64-unknown-uefi").join(profile).join("keefeos-bootloader.efi");

    // Verify binaries exist
    if !kernel_bin.exists() {
        eprintln!("Kernel binary not found: {:?}", kernel_bin);
        eprintln!("Run 'xtask build' first");
        exit(1);
    }
    if !bootloader_efi.exists() {
        eprintln!("Bootloader EFI binary not found: {:?}", bootloader_efi);
        eprintln!("Run 'xtask build' first");
        exit(1);
    }

    // Create image file
    let image_path = target_dir.join("keefeos.img");
    let image_size = 64 * 1024 * 1024; // 64MB

    // Create a zero-filled image file
    let mut image_file = File::create(&image_path).expect("Failed to create image file");
    image_file.set_len(image_size as u64).expect("Failed to set image size");
    drop(image_file);

    // Format as FAT32 and copy files using fatfs crate
    format_fat_image(&image_path, &kernel_bin, &bootloader_efi).expect("Failed to format FAT image");

    println!("Disk image created: {:?}", image_path);
}

fn format_fat_image(image_path: &Path, kernel_bin: &Path, bootloader_efi: &Path) -> std::io::Result<()> {
    use fatfs::{FileSystemOptions, FsOptions, LossyOemCpConverter};

    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(image_path)?;

    let options = FsOptions::new()
        .volume_label(*b"KEEFEOS    ")
        .oem_cp_converter(Box::new(LossyOemCpConverter::new()));
    
    let fs = fatfs::format_volume(&file, options)?;
    
    // Create EFI directory structure
    let root_dir = fs.root_dir();
    let efi_dir = root_dir.create_dir("EFI")?;
    let boot_dir = efi_dir.create_dir("BOOT")?;
    
    // Copy bootloader to EFI/BOOT/BOOTX64.EFI
    let mut bootloader_data = Vec::new();
    File::open(bootloader_efi)?.read_to_end(&mut bootloader_data)?;
    let mut bootloader_file = boot_dir.create_file("BOOTX64.EFI")?;
    bootloader_file.write_all(&bootloader_data)?;
    
    // Copy kernel to root directory
    let mut kernel_data = Vec::new();
    File::open(kernel_bin)?.read_to_end(&mut kernel_data)?;
    let mut kernel_file = root_dir.create_file("KERNEL.ELF")?;
    kernel_file.write_all(&kernel_data)?;

    Ok(())
}

fn run_qemu(_release: bool) {
    println!("Launching QEMU...");

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
    let image_path = workspace_root.join("target").join("keefeos.img");

    if !image_path.exists() {
        eprintln!("Disk image not found: {:?}", image_path);
        eprintln!("Run 'xtask build' first");
        exit(1);
    }

    // Check for OVMF firmware
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

    let ovmf_code = ovmf_code_paths.iter().find(|p| Path::new(p).exists())
        .unwrap_or(&"/usr/share/OVMF/OVMF_CODE.fd");
    let ovmf_vars = ovmf_vars_paths.iter().find(|p| Path::new(p).exists())
        .unwrap_or(&"/usr/share/OVMF/OVMF_VARS.fd");

    let status = Command::new("qemu-system-x86_64")
        .args([
            "-drive", &format!("if=pflash,format=raw,readonly=on,file={}", ovmf_code),
            "-drive", &format!("if=pflash,format=raw,file={}", ovmf_vars),
            "-drive", &format!("format=raw,file={}", image_path.display()),
            "-serial", "stdio",
            "-m", "256M",
            "-cpu", "qemu64",
        ])
        .status()
        .expect("Failed to start QEMU");

    if !status.success() {
        eprintln!("QEMU exited with status: {}", status);
    }
}
