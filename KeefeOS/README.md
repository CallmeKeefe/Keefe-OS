# KeefeOS — experimental Rust operating-system prototype

KeefeOS is an experimental x86-64 bare-metal kernel and UEFI loader. One early boot path has been verified in QEMU/OVMF, but this is not a general-purpose or production-ready operating system.

## Verified status

GitHub Actions run [40201424209](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201424209) passed 22 boot-protocol tests, 31 kernel unit tests, both builds, image packaging, and the QEMU/OVMF serial smoke test at commit `98f1830908c57861910df866a51dfafa138b0d04`. The firmware accepted the GOP BLT calls for a readable pre-boot splash, and the kernel emitted `KEEFEOS-KERNEL-BOOT-OK` after validating BootInfo.

The tested kernel uses a **fixed 32–64 MiB x86-64 load window**. Relocation is not implemented. The UEFI splash uses firmware BLT before exiting boot services. The post-boot framebuffer console is still fail-closed because the final memory map has a gap over the GOP range. This does not prove support for physical hardware or every UEFI implementation. ACPI, PCI enumeration, a broad driver stack, multi-architecture support, secure boot, real cryptography, and user-space isolation remain incomplete.

## Workspace packages

- `boot-protocol`: versioned BootInfo and bounds-checked ELF64 parsing.
- `bootloader`: UEFI application and ELF segment loading.
- `kernel`: x86-64 bare-metal kernel and experimental subsystems.
- `xtask`: host-side image and QEMU helper.

## Build and test

Use Rust nightly with `rust-src`, `llvm-tools`, `rustfmt`, `clippy`, and the `x86_64-unknown-none` / `x86_64-unknown-uefi` targets. Linux or WSL2 is the documented build and QEMU environment.

```sh
cargo +nightly fmt --all -- --check
cargo +nightly test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo +nightly test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

From the repository root, run the full QEMU smoke test via GitHub Actions or the documented QEMU setup. A successful build/image generation does not itself establish a boot.

## Security

The security APIs are a prototype interface, not a security boundary. Cryptography, trusted random generation, signature verification, vulnerability scanning and attack detection are not implemented as reliable controls. See root `SECURITY.md` and `docs/audit/SECURITY_STATUS.md`.
