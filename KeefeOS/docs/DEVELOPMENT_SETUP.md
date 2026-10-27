# KeefeOS development setup

This guide covers source builds, tests, and a disposable QEMU image. It does not describe an end-user installation.

## Verified test path

[GitHub Actions run 40602581622](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40602581622) on audit-branch commit `67d692063c37c975e3799c55a8e1538d36058b04` (PR #27; not merged at the time of this evidence) passed the boot-protocol tests and kernel tests, the host helper check, both target builds, image packaging, and QEMU/OVMF boot and splash checks at 256 MiB, 512 MiB, and 1 GiB RAM. Each profile logged `UEFI GOP splash drawn through firmware BLT` and `KEEFEOS-KERNEL-BOOT-OK`.

This is still a single QEMU/OVMF firmware/device setup with varying RAM sizes. Physical machines, other UEFI implementations, native Windows/macOS hosts, ARM64, RISC-V, phones, and other hypervisors are not verified. The loader uses a fixed 32–64 MiB x86-64 kernel load window, not a relocatable image.

## Prerequisites

Use Linux or WSL2 with Ubuntu. Install Rust nightly and targets:

```sh
rustup toolchain install nightly-2026-10-27 --profile minimal --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

## Build and tests

From the repository root:

```sh
cd KeefeOS
cargo fmt --all -- --check
cargo test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

CI runs the QEMU memory matrix and fails unless every profile renders the UEFI splash through BLT and reaches the kernel boot marker. Logs are uploaded per RAM size.

## Known limitations

- Fixed kernel load address; no dynamic relocation.
- Full higher-half/page-table setup and process isolation are not verified.
- Firmware UEFI BLT draws the boot splash; the kernel's direct framebuffer console remains disabled due to a gap in the final memory map.
- Build warnings remain and need triage.
- Security/AI/scanner/attack-detection modules are not production protections.

Do not install this prototype on physical hardware or use disks containing valuable data.
