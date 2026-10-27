# KeefeOS testing guide

**KeefeOS is not an installable end-user OS.** Do not use these commands to install on physical hardware, dual-boot, or write an image to a disk containing data.

The latest passing audit-branch CI run [40602581622](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40602581622) on commit `67d692063c37c975e3799c55a8e1538d36058b04` (PR #27; not merged at the time of this evidence) passed x86-64 QEMU/OVMF boots at 256 MiB, 512 MiB, and 1 GiB RAM, each showing the firmware-rendered GOP splash and kernel serial marker. These are development tests, not proof of physical-device compatibility. The kernel's direct framebuffer console remains disabled because the final memory map has a gap over GOP memory.

## Development environment

Use Linux or WSL2 with Rust nightly, QEMU, and OVMF:

```sh
rustup toolchain install nightly-2026-10-27 --profile minimal --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

From repository root:

```sh
cd KeefeOS
cargo test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

For the reproducible RAM matrix, use the GitHub Actions workflow, which starts separate QEMU/OVMF instances with independent OVMF variable copies and captures one serial log per RAM profile.

## Compatibility limitations

- Tested: x86-64 QEMU + OVMF UEFI with 256 MiB, 512 MiB and 1 GiB RAM.
- Not tested: physical x86-64 hardware, VirtualBox, VMware, Hyper-V, or alternative UEFI implementations.
- Not supported: legacy BIOS, ARM64, RISC-V, Apple Silicon as native target, phones/tablets, consoles.
- Not implemented: end-user installer, disk partitioning workflow, upgrades, recovery environment, persistent kernel graphics console, broad driver stack, safe production configuration.

See [the compatibility matrix](../../docs/COMPATIBILITY.md) and [OS status](../../docs/audit/OS_STATUS.md).
