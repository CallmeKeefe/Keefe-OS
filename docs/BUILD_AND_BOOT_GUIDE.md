# KeefeOS build and boot guide

**Status: experimental prototype. This guide builds the project and boots it in QEMU; it does not install KeefeOS on a physical computer.**

KeefeOS currently verifies an x86-64 UEFI early-boot path under QEMU + OVMF. It is not a complete end-user operating system: there is no installer, recovery environment, upgrade mechanism, general storage/input/network driver stack, or production security boundary. Do not flash this image to a USB drive, install it to a physical disk, or use it with data you need to preserve.

## 1. Know what is and is not supported

| Host or target | Status |
|---|---|
| Ubuntu Linux x86-64 host building the project | Documented workflow; CI uses Ubuntu |
| Windows with WSL2 running Ubuntu | Reasonable build-host path; use QEMU/OVMF inside WSL2 where available; host-specific display/virtualization behavior may vary |
| QEMU x86-64 + OVMF guest with 256 MiB, 512 MiB, or 1 GiB RAM | Early boot verified in the cited CI run |
| Native Windows, macOS, Apple Silicon, Android/iOS | Not verified as build hosts or supported boot targets |
| Physical x86-64 computers, other hypervisors, legacy BIOS | Not supported/verified for installation |
| ARM64, RISC-V, phones, consoles, embedded boards | Not supported |

The latest confirmed CI evidence is listed in [docs/COMPATIBILITY.md](COMPATIBILITY.md); do not assume it covers later main-branch commits. A QEMU boot marker proves only the tested early-boot milestone, not a complete OS or working device drivers.

## 2. Prepare an Ubuntu build environment

Use Ubuntu 22.04/24.04 or an Ubuntu WSL2 distribution. Open a terminal and install host dependencies:

```sh
sudo apt-get update
sudo apt-get install -y build-essential curl git qemu-system-x86 ovmf
```

Install Rustup if it is not already installed. Follow the official instructions at https://rustup.rs/ and restart the terminal if the installer asks you to update your PATH.

Install the nightly compiler, required components, and bare-metal targets:

```sh
rustup toolchain install nightly-2026-10-27 --profile minimal \
  --component rust-src \
  --component llvm-tools \
  --component rustfmt \
  --component clippy \
  --target x86_64-unknown-none \
  --target x86_64-unknown-uefi
```

Confirm the tools are available:

```sh
rustup toolchain list
cargo --version
qemu-system-x86_64 --version
```

## 3. Get the source

```sh
git clone https://github.com/CallmeKeefe/Keefe-OS.git
cd Keefe-OS/KeefeOS
git status --short
```

For reproducible checks, start from a commit whose CI result you intend to reproduce. The repository's default branch may contain newer work than the milestone cited in the compatibility document.

## 4. Run formatting and host-side tests

Run these commands from the `KeefeOS/` workspace directory:

```sh
cargo fmt --all -- --check
cargo test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo check --locked --package xtask --target x86_64-unknown-linux-gnu
```

A failure is a stop condition. Do not treat later image creation as proof that the tests passed.

## 5. Build the kernel, UEFI loader, and disk image

Still from `KeefeOS/`:

```sh
cargo build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
```

The image command assembles the project image. **Image creation is not a boot test and is not an installer.** Keep the generated image as an experimental artifact.

## 6. Verify early boot in QEMU/OVMF

The latest completed passing code run confirmed from GitHub records is [Actions #40698180312](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40698180312) on PR-head commit `9cfbbff013765ca88b3a999471ab2248787b1308`. It passed 22 boot-protocol tests, 51 kernel unit tests, the RustSec audit, Clippy, both target builds, image packaging, and the QEMU/OVMF three-profile boot matrix. The main branch later advanced to `c27a616c7330702c65213f185a0dec41810a6b4d`; that newer code and subsequent workflow/documentation changes need their own confirmed result. Check [current Actions runs](https://github.com/CallmeKeefe/Keefe-OS/actions). This boot evidence does not prove a complete OS or physical-device support.

The canonical end-to-end test is the repository's [KeefeOS CI workflow](../.github/workflows/keefeos-ci.yml). It builds and packages the image, discovers a matching OVMF firmware code/variables pair, and boots QEMU at 256 MiB, 512 MiB, and 1 GiB RAM. The workflow requires both the firmware splash marker and `KEEFEOS-KERNEL-BOOT-OK` in each serial log.

To run the same end-to-end test locally, inspect the current workflow and reproduce its QEMU matrix rather than guessing firmware paths: distributions package OVMF files under different names and directories. Use a writable copy of the OVMF variables file for each run. If you change the bootloader, memory map, kernel, or QEMU arguments, rerun all three profiles.

Check the latest [Actions runs](https://github.com/CallmeKeefe/Keefe-OS/actions) for the exact commit and result. A successful build with a failed or missing boot marker is a failed boot test.

## 7. Troubleshooting

- **Rust target missing:** rerun the `rustup toolchain install nightly-2026-10-27 --profile minimal ...` command above and check `rustup target list --toolchain nightly --installed`.
- **QEMU command not found:** install `qemu-system-x86` using the package manager for the Ubuntu environment you are actually using.
- **OVMF files not found:** install `ovmf`; inspect `/usr/share/OVMF`, `/usr/share/ovmf`, and `/usr/share/qemu` for a matching firmware code/variables pair. Do not pair unrelated firmware files.
- **QEMU cannot use virtualization acceleration:** use software emulation for this test; do not change host firmware or security settings just to make the prototype run.
- **No boot marker:** save the serial log and record the exact commit, QEMU version, firmware file pair, RAM profile, and command. Do not report the test as passed.
- **WSL2 display/firmware issue:** first validate compilation and host tests. QEMU integration can vary by Windows/WSL version; use the Linux CI run as the canonical reference.
- **Build works but device is unusable:** expected at this stage. General storage, USB, input, network, power management, and installer/recovery support are not implemented.

## 8. Physical installation and recovery: not available

Do not follow generic OS installation instructions to write the image to a USB stick or disk. KeefeOS does not provide a supported physical installer, verified hardware compatibility list, recovery environment, upgrade process, or safe disk-layout management. Installing it on real hardware could make existing data inaccessible.

A future physical-device milestone must first include a tested installer/image layout, documented supported hardware, safe disk selection and confirmation, recovery/rollback procedures, reproducible builds, signed artifacts, and successful non-destructive hardware tests. None of those should be implied by the current QEMU early-boot result.

## 9. Security and feature status\n\nA pinned CodeQL workflow now targets Rust and GitHub Actions under `../.github/workflows/codeql.yml`; its first scan has not yet been verified. A configured scanner is not a clean scan.

Secure-boot signature verification, trusted cryptographic operations, capability verification, real vulnerability scanning, and attack detection are not implemented as reliable security controls. An empty scanner result does not mean a system is clean. See [SECURITY.md](../SECURITY.md), [security status](audit/SECURITY_STATUS.md), and [OS status](audit/OS_STATUS.md).

## 10. Report a reproducible failure

When opening an issue, include:
- the exact commit SHA and host OS/version;
- Rust, QEMU, and OVMF versions;
- the RAM profile and exact command;
- the serial log and failed CI job link;
- whether the failure reproduces on the repository's Ubuntu CI workflow.

Do not include passwords, API tokens, private keys, or personal information in logs.
