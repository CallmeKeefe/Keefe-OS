# KeefeOS development setup

This guide covers source builds, tests, and a disposable QEMU image. It does not describe an end-user installation.

## Verified test path

One x86-64 QEMU + OVMF UEFI boot passed in [Actions run 40199251364](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40199251364) at commit `2683ea911db964a68f86c615372fc13171bf3c4e`. The serial output showed BootInfo validation and `KEEFEOS-KERNEL-BOOT-OK`.

This is a single emulator configuration. Physical machines, other UEFI implementations, native Windows/macOS hosts, ARM64, RISC-V, phones, and other hypervisors are not verified. The loader currently uses a fixed 32–64 MiB x86-64 kernel load window rather than a relocatable image.

## Prerequisites

Use Linux or WSL2 with Ubuntu for the documented image/QEMU path. Install Rust nightly and required targets:

```sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

## Build and tests

Run from the repository root:

```sh
cd KeefeOS
cargo +nightly fmt --all -- --check
cargo +nightly test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo +nightly test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo +nightly check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

Build the kernel before the loader because the loader embeds the kernel artifact. Keep profiles aligned; pass `--release` to each relevant build and image command for release artifacts.

## QEMU/OVMF smoke test

The GitHub Actions workflow is the authoritative tested path. It discovers OVMF firmware, assembles the FAT image, launches QEMU, captures serial output, and fails unless `KEEFEOS-KERNEL-BOOT-OK` is present. The run uses a disposable copy of the OVMF variable-store image.

## Known limitations

- The kernel load address is fixed, not dynamically relocatable.
- The page-table/higher-half work is still experimental; the current smoke test uses the firmware-provided address space through early boot.
- GOP framebuffer output and device drivers are not implemented; current diagnostic output is serial.
- Build warnings remain and require cleanup.
- Security, AI, scanner, and attack-detection modules are experimental placeholders; do not rely on them as protections.

Do not install this prototype on physical hardware or test it against drives containing valuable data.
