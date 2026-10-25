# KeefeOS

[![CI](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml/badge.svg?branch=main)](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/blank.yml) ![Experimental](https://img.shields.io/badge/status-experimental-orange) ![Architecture x86-64](https://img.shields.io/badge/architecture-x86--64-blue) ![Security incomplete](https://img.shields.io/badge/security-incomplete-red)

> **Experimental OS prototype.** An early kernel boot has been verified in QEMU/OVMF, but physical hardware installation and production use are not supported. The kernel uses a fixed physical load window and only one emulator configuration has been validated.

## Latest verified milestone

**PASS:** [GitHub Actions run 40201424209](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201424209), commit `98f1830908c57861910df866a51dfafa138b0d04`.

The run passed 22 boot-protocol/ELF tests, 31 kernel unit tests, the kernel and UEFI-loader builds, disk-image packaging, and the QEMU/OVMF serial smoke test. The firmware accepted all GOP BLT operations used to draw a readable boot splash before exiting boot services. The serial log reported:

```text
UEFI GOP splash drawn through firmware BLT
GOP candidate: 0x80000000, 4096000 bytes, 1280x800, stride 1280
GOP range has a gap in the final memory map; unsafe framebuffer writes refused
KEEFEOS-KERNEL-BOOT-OK
```

This verifies one x86-64 QEMU + OVMF boot path and a firmware-rendered splash. The kernel's persistent framebuffer console remains disabled because the final UEFI memory map does not fully describe the framebuffer range; serial remains the authoritative kernel diagnostic channel. It does **not** establish physical-device support, a general-purpose OS, secure boot, working user-space isolation, or production security. Build warnings remain and should be reduced.

## Current engineering status

| Area | Status |
|---|---|
| Shared BootInfo ABI and bounds-checked ELF64 parser | Implemented; host tests pass in the verified run |
| Kernel/UEFI loader builds | Passed in the verified run |
| QEMU/OVMF early boot | **Passed** for one QEMU x86-64 configuration |
| Kernel load address | Fixed 32–64 MiB identity-mapped window; relocation is not implemented |
| Memory map | Versioned BootInfo with structural/semantic checks; broader firmware edge cases still need testing |
| Physical frame allocator | Regression tests added; only the emulator path has been exercised |
| UEFI visual boot splash | PASS in QEMU via firmware BLT before ExitBootServices |\n| Persistent kernel framebuffer console | Safely disabled; final UEFI map has a gap covering the GOP range, so direct writes are refused |
| ACPI, PCI, storage, USB, network, input and power management | Not implemented as a general driver stack |
| Secure boot, authenticated cryptography, capability signatures | Not implemented; security-sensitive interfaces fail closed |
| Vulnerability scanner / attack detection | Placeholder APIs; empty output does not mean a clean system |
| ARM64, RISC-V, mobile devices | Not supported |

Check [Actions](https://github.com/CallmeKeefe/Keefe-OS/actions) for the latest result. A later commit may have a different status from the run cited above.

## Repository layout

- `KeefeOS/boot-protocol` — shared versioned BootInfo ABI and bounds-checked ELF64 parser.
- `KeefeOS/bootloader` — UEFI loader for the constrained x86-64 ELF image.
- `KeefeOS/kernel` — x86-64 kernel, memory management, interrupt setup and experimental modules.
- `KeefeOS/xtask` — host-side build, FAT image packaging and QEMU helper.
- `docs/audit/OS_STATUS.md` — current status and evidence.
- `docs/COMPATIBILITY.md` — platform compatibility matrix.
- `docs/audit/SECURITY_STATUS.md` and `SECURITY.md` — security limitations and reporting policy.
- `docs/QWEN_CODER_PROMPT.md` — engineering handoff prompt.

## Build and test (Linux / WSL2)

Install the nightly toolchain and targets:

```sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

From the workspace directory:

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

CI additionally boots the generated image in QEMU/OVMF and requires `KEEFEOS-KERNEL-BOOT-OK` on serial. Do not claim boot success unless that step passes.

## Compatibility

The tested configuration is x86-64 QEMU + OVMF UEFI. Physical x86-64 devices, VirtualBox, VMware, Hyper-V, native Windows/macOS hosts, ARM64, RISC-V, phones, and consoles have not been validated as boot targets. See [the compatibility matrix](docs/COMPATIBILITY.md).

## Security posture

KeefeOS is not a security boundary. Secure-boot signatures, trusted randomness, authenticated encryption, a real capability verifier, runtime vulnerability scanning and attack detection remain unimplemented. Their APIs fail closed or report incomplete status rather than pretending to protect the system.

Read [SECURITY.md](SECURITY.md), [the security status](docs/audit/SECURITY_STATUS.md), and [the initial codebase audit](docs/audit/INITIAL_CODEBASE_AUDIT.md).

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md). Prioritize boot correctness, memory safety, regression tests, and evidence-based documentation over speculative feature claims.
