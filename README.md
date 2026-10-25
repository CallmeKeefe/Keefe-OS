# KeefeOS

[![CI](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml/badge.svg?branch=main)](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml) ![Experimental](https://img.shields.io/badge/status-experimental-orange) ![x86--64 only](https://img.shields.io/badge/architecture-x86--64-blue) ![Security incomplete](https://img.shields.io/badge/security-incomplete-red)

KeefeOS is an experimental Rust operating-system project targeting an x86-64 bare-metal kernel with a UEFI boot path. It is an engineering prototype, not a production-ready operating system.

## Project status

| Area | Current status |
|---|---|
| Rust workspace and kernel source | Present |
| x86-64 bare-metal target configuration | Present; build must be verified by CI |
| UEFI bootloader source | Present |
| Disk-image packaging helper | Present |
| Automated compile/package workflow | Added; check the latest Actions run for its actual result |
| Successful firmware-to-kernel boot | **Not verified** |
| Memory-map ABI and physical/virtual mapping | Known issues remain |
| User-space processes, IPC, production driver stack | Not claimed as complete |
| Security certification or independent audit | Not performed |

This status is intentionally conservative: source files, roadmap statements, module names, and startup messages are not proof of working OS features.

## Hardware compatibility

KeefeOS is currently an x86-64 experimental prototype. It is not compatible with every device and is not yet installable. Read the [compatibility matrix](docs/COMPATIBILITY.md) for supported build targets, unverified firmware paths, and platforms that are not supported.

## Repository layout

- KeefeOS/kernel — kernel binary/library plus architecture, memory, interrupt, AI, and security modules.
- KeefeOS/bootloader — UEFI application and kernel-image embedding build script.
- KeefeOS/xtask — host-side build orchestration, FAT image creation, and QEMU launcher.
- KeefeOS/docs — development setup, installation/testing notes, and contributor guidance.
- docs/audit — evidence-based audit and branch-review records.
- docs/QWEN_CODER_PROMPT.md — detailed handoff prompt for Qwen Coder.
- NEXUS_OS_*.md — architecture proposals and roadmap, not a guarantee that every proposal is implemented.

## Quick start (Linux or WSL2)

The current helper assumes a Linux host with QEMU and OVMF. Windows developers should use WSL2 for the documented image/QEMU flow.

Install Rust nightly with the components and targets used by the workspace:

~~~sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
~~~

On Ubuntu/Debian, install the emulator and UEFI firmware packages:

~~~sh
sudo apt update
sudo apt install qemu-system-x86 ovmf
~~~

Build from the workspace directory:

~~~sh
cd KeefeOS
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
~~~

The kernel and bootloader must use the same Cargo profile. For a release image, add --release to both build commands and to the xtask image command. The bootloader embeds the kernel artifact for the selected profile.

To build the image and launch QEMU in one command:

~~~sh
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- run
~~~

The run command depends on OVMF firmware being available in one of the Linux paths checked by the helper. The current automation does not yet provide a verified boot result.

## Important boot limitation

The current bootloader passes an ELF kernel image through UEFI's LoadImage path and uses the loaded image base as if it were the kernel's entry point. A generic x86_64-unknown-none ELF kernel is not a UEFI application image. The kernel also uses a hard-coded UEFI memory-map descriptor layout that does not consistently match the descriptor ABI. These are boot blockers; a successful compile or disk-image build does **not** demonstrate successful boot.

See [the initial codebase audit](docs/audit/INITIAL_CODEBASE_AUDIT.md), [branch consolidation report](docs/audit/BRANCH_CONSOLIDATION_REPORT.md), and [development setup guide](KeefeOS/docs/DEVELOPMENT_SETUP.md).

## Development and contributions

Read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting changes. Prioritize reproducible builds, correct boot and memory handling, narrow unsafe boundaries, regression tests, and evidence-backed documentation over speculative features.

## License metadata

The Cargo workspace currently declares MIT OR Apache-2.0, but the canonical license text files are not present in the current repository tree. This mismatch needs to be resolved before publishing or redistributing release artifacts.
