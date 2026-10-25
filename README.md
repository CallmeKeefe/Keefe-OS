# KeefeOS

[![CI](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml/badge.svg?branch=main)](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml) ![Experimental](https://img.shields.io/badge/status-experimental-orange) ![x86--64](https://img.shields.io/badge/architecture-x86--64-blue) ![Security incomplete](https://img.shields.io/badge/security-incomplete-red)

> **Experimental OS prototype.** Physical hardware installation and production use are not supported. A successful build does not prove the image boots.

## Current engineering status

| Area | Status |
|---|---|
| Shared BootInfo and ELF64 parser | Implemented; host tests pass in the last observed CI run |
| Kernel target build | Passed in the last observed CI run |
| UEFI loader build | A loader integration attempt is being validated in CI |
| ELF64 loading and BootInfo handoff | Implemented in source; runtime boot not yet verified |
| QEMU/OVMF serial smoke test | Added to CI; must observe `KEEFEOS-KERNEL-BOOT-OK` to pass |
| Physical frame allocator | Regression tests added; real firmware behavior still needs runtime validation |
| Secure boot, authenticated cryptography, capability signatures | Not implemented; security-sensitive interfaces fail closed |
| Vulnerability scanning / attack detection | Placeholder APIs; an empty result is not evidence of security |
| ARM64, RISC-V, mobile devices | Not supported |

See [GitHub Actions](https://github.com/CallmeKeefe/Keefe-OS/actions) for the latest build and emulation outcomes.

## Repository layout

- `KeefeOS/boot-protocol` — shared versioned BootInfo ABI and bounds-checked ELF64 parser.
- `KeefeOS/bootloader` — UEFI loader for the constrained x86-64 ELF image.
- `KeefeOS/kernel` — x86-64 kernel, memory management, interrupt setup, and experimental modules.
- `KeefeOS/xtask` — host-side build, FAT image packaging, and QEMU helper.
- `docs/audit` — evidence-based code/security/branch audit.
- `docs/COMPATIBILITY.md` — known platform compatibility.
- `docs/QWEN_CODER_PROMPT.md` — engineering handoff prompt.

## Build and test (Linux / WSL2)

Install nightly Rust and the bare-metal targets:

~~~sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
~~~

From the workspace directory:

~~~sh
cd KeefeOS
cargo +nightly fmt --all -- --check
cargo +nightly test --locked --package keefeos-boot-protocol --target x86_64-unknown-linux-gnu
cargo +nightly test --locked --package keefeos-kernel --lib --target x86_64-unknown-linux-gnu
cargo +nightly check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
~~~

The CI job additionally attempts QEMU/OVMF and requires the kernel's serial boot marker. Do not claim successful boot unless that step passes. Native Windows, macOS, ARM64, RISC-V, phones, and physical-device installation are not supported.

## Security posture

KeefeOS is not a security boundary. Secure-boot signatures, trusted randomness, authenticated encryption, a real capability verifier, runtime vulnerability scanning, and attack detection remain unimplemented. Read [SECURITY.md](SECURITY.md) and [the security status](docs/audit/SECURITY_STATUS.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Prioritize boot correctness, memory safety, regression tests, and evidence-based documentation over speculative feature claims.
