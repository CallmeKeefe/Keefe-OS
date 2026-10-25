# KeefeOS testing guide

**KeefeOS is not currently an installable end-user OS.** Do not use the commands in this guide to install it on physical hardware, dual-boot it, or write an image to a disk containing data.

One x86-64 QEMU + OVMF UEFI boot path passed in [GitHub Actions run 40201424209](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201424209). The run also confirms the pre-boot GOP splash is drawn through firmware BLT. The kernel's post-boot direct framebuffer remains disabled due to a gap in the final memory map. The build and smoke test are for development only. This does not establish compatibility with physical PCs, every UEFI implementation, or other architectures.

## Supported test environment

The documented path is Linux or WSL2 with Rust nightly, QEMU, and OVMF.

```sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

From repository root:

```sh
cd KeefeOS
cargo +nightly test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo +nightly test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

For the exact QEMU smoke test, use the GitHub Actions workflow linked above; it captures serial output and requires the kernel boot milestone.

## Compatibility limitations

- Tested: x86-64 QEMU + OVMF UEFI only.
- Not tested: physical x86-64 hardware, VirtualBox, VMware, Hyper-V, or other UEFI implementations.
- Not supported: legacy BIOS, ARM64, RISC-V, Apple Silicon as a native boot target, phones/tablets, and consoles.
- Not implemented: end-user installer, disk partitioning/formatting workflow, upgrade process, recovery environment, persistent kernel graphics console, broad device driver stack, or safe production configuration.

See [the compatibility matrix](../../docs/COMPATIBILITY.md) and [OS status](../../docs/audit/OS_STATUS.md) for evidence and remaining work.
