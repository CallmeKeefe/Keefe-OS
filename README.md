# KeefeOS

KeefeOS is an experimental Rust operating-system project targeting an x86-64 bare-metal kernel and UEFI boot path. It is an engineering prototype, not a production-ready operating system.

## Repository layout

- `KeefeOS/kernel` — kernel binary/library and memory, architecture, interrupt, AI, and security modules.
- `KeefeOS/bootloader` — UEFI application.
- `KeefeOS/xtask` — host-side build, image packaging, and QEMU helper.
- `KeefeOS/docs` — installation and contributor documentation.
- `NEXUS_OS_*.md` — architecture proposals and roadmap material.

## Build prerequisites

Use the pinned nightly Rust channel with `rust-src` and the `x86_64-unknown-none` and `x86_64-unknown-uefi` targets. QEMU and OVMF are needed to attempt emulation.

From the repository root:

```sh
cd KeefeOS
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

Build kernel and bootloader with the same profile. Use `--release` for both if packaging a release-profile image.

## Important limitations

A successful compilation or packaged disk image does not prove that firmware can load the kernel or that the OS boots. The current boot handoff still passes a generic ELF kernel to UEFI `LoadImage` and has an inconsistent memory-map descriptor ABI. Until these are repaired and the image is boot-tested, bootability is **not verified**.

Security, capability, AI/ML, scheduler, encryption, and distributed-system claims must be judged against implementation and tests, not roadmap text or startup banners.

See [the initial codebase audit](docs/audit/INITIAL_CODEBASE_AUDIT.md) and [branch consolidation report](docs/audit/BRANCH_CONSOLIDATION_REPORT.md) for the current evidence and remaining work.
