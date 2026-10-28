# KeefeOS — experimental Rust operating-system prototype

KeefeOS is an experimental x86-64 bare-metal kernel and UEFI loader. Early boot has been verified only in QEMU/OVMF, not as a general-purpose or production-ready OS.

## Latest verified status

The latest completed passing run confirmed from GitHub records is [Actions run 40698180312](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40698180312) on PR-head commit `9cfbbff013765ca88b3a999471ab2248787b1308`. It passed 22 boot-protocol tests and 51 kernel unit tests, the host helper check, RustSec cargo-audit, Clippy, optional legacy-VGA compilation, both target builds, image packaging, and QEMU/OVMF boot/splash tests at 256 MiB, 512 MiB, and 1 GiB RAM. The kernel emitted `KEEFEOS-KERNEL-BOOT-OK` in all three profiles. This proves only that exact commit: later main code `c27a616c7330702c65213f185a0dec41810a6b4d` and subsequent workflow/doc commits still need confirmed CI runs.

The loader draws a UEFI GOP splash using firmware BLT before ExitBootServices. Direct kernel framebuffer writes remain disabled until active page-table mapping and MMIO cache attributes are proven; a reserved UEFI memory descriptor alone is insufficient. The kernel is linked for a fixed **32–64 MiB x86-64 load window**; relocation is not implemented.

A CodeQL workflow for Rust and GitHub Actions is configured in `.github/workflows/codeql.yml`; its first scan result is not yet verified.\n\n## Workspace packages

- `boot-protocol`: versioned BootInfo and bounded ELF64 parser.
- `bootloader`: UEFI application and kernel segment loading.
- `kernel`: x86-64 kernel and experimental subsystem APIs.
- `xtask`: host-side image and QEMU helper.

## Build and test

Use Rust nightly with `rust-src`, `llvm-tools`, `rustfmt`, `clippy`, `x86_64-unknown-none`, and `x86_64-unknown-uefi`. Linux/WSL2 with QEMU and OVMF is the current development path.

```sh
cargo fmt --all -- --check
cargo test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

CI performs the actual QEMU/OVMF boot matrix. A successful build or image generation is not by itself evidence of a boot.

## Security and compatibility

Security APIs are prototypes, not a security boundary. Real cryptography, trusted randomness, signature verification, vulnerability scanning, and attack detection are not implemented as reliable controls. Hardware support is x86-64 QEMU/OVMF only for the cited memory profiles; physical hardware, ARM64, RISC-V, phones, and other hypervisors are not verified. See root `README.md`, `CONTRIBUTING.md`, and `docs/audit/SECURITY_STATUS.md`.
