# KeefeOS

[![CI](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/keefeos-ci.yml/badge.svg?branch=main)](https://github.com/CallmeKeefe/Keefe-OS/actions/workflows/keefeos-ci.yml) ![Experimental](https://img.shields.io/badge/status-experimental-orange) ![Architecture x86-64](https://img.shields.io/badge/architecture-x86--64-blue) ![Security incomplete](https://img.shields.io/badge/security-incomplete-red)

> **Experimental OS prototype.** Early boot is verified only under x86-64 QEMU + OVMF UEFI. Physical installation and production use are not supported. The kernel uses a fixed load window; the persistent kernel framebuffer console remains disabled until target-specific page-table mapping and MMIO cache attributes are proven; a reserved UEFI memory descriptor alone is insufficient.

## Latest verified milestone

**Most recent completed full CI pass before consolidation:** [run 40606689369](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40606689369), commit `adc98ccdd429cb787a7676133fdb95a1540a375c`. PR #27 was closed in favor of consolidated [PR #28](https://github.com/CallmeKeefe/Keefe-OS/pull/28); check PR #28 for the current run.

The run passed 22 boot-protocol tests and 47 kernel unit tests, the host helper check, RustSec `cargo-audit` against the locked dependency set, Clippy for the boot protocol/kernel/helper, the optional legacy-VGA compile check, kernel and UEFI-loader builds, image packaging, and the QEMU/OVMF boot matrix at **256 MiB, 512 MiB, and 1 GiB RAM**. Each profile required the firmware-rendered GOP splash and `KEEFEOS-KERNEL-BOOT-OK`. Direct kernel framebuffer writes remain disabled until active mapping and MMIO cache attributes are proven.

Representative serial output:

```text
PASS: 256 MiB RAM boot marker and GOP splash verified.
UEFI GOP splash drawn through firmware BLT
GOP range has a gap in the final memory map; unsafe framebuffer writes refused
KEEFEOS-KERNEL-BOOT-OK
PASS: 512 MiB RAM boot marker and GOP splash verified.
PASS: 1024 MiB RAM boot marker and GOP splash verified.
```

This verifies three QEMU RAM profiles using the same x86-64 QEMU/OVMF setup. It does **not** prove support for physical computers, different firmware implementations or hypervisors, universal hardware compatibility, secure boot, full user-space isolation, or production security. Compiler warnings remain.

## Current engineering status

| Area | Status |
|---|---|
| BootInfo ABI and bounded ELF64 parser | 22 protocol tests passed |
| Kernel unit tests | 47 passed |
| Kernel / UEFI loader builds | Passed in the cited CI run |
| QEMU/OVMF early boot | Passed with 256 MiB, 512 MiB, and 1 GiB RAM |
| UEFI visual boot splash | Passed using firmware GOP BLT before ExitBootServices |
| Kernel load address | Fixed 32–64 MiB identity-mapped window; relocation is not implemented |
| Direct kernel framebuffer console | **Disabled by design**: a reserved UEFI descriptor does not prove active virtual mapping or correct MMIO cache attributes; firmware draws the splash before `ExitBootServices`. |
| Physical frame allocator | Regression tests exist; firmware-map and runtime stress coverage remain limited |
| Paging / higher-half model | Experimental; full page-table lifecycle and isolation are not verified |
| ACPI, PCI, storage, USB, network, input, and power management | No general driver stack |
| Secure boot, authenticated cryptography, capability signatures | Not implemented; sensitive APIs fail closed |
| Vulnerability scanner / attack detector | Placeholder interfaces; empty results do not mean a clean system |
| ARM64, RISC-V, mobile devices | Not supported |
| End-user installer / recovery / upgrades | Not implemented |

Check [GitHub Actions](https://github.com/CallmeKeefe/Keefe-OS/actions) for the latest result. Results from later commits may differ.

## Repository layout

- `KeefeOS/boot-protocol` — versioned BootInfo ABI and bounds-checked ELF64 parser.
- `KeefeOS/bootloader` — UEFI loader and firmware-rendered boot splash.
- `KeefeOS/kernel` — x86-64 kernel, memory management, interrupt setup, and experimental modules.
- `KeefeOS/xtask` — host build, FAT image packaging, and QEMU helper.
- `docs/BUILD_AND_BOOT_GUIDE.md` — step-by-step host setup, build, test, and QEMU boot guide; explicitly not a physical installer.
- `docs/COMPATIBILITY.md` — current hardware/platform compatibility matrix.
- `docs/audit/OS_STATUS.md` — latest tested state and remaining risks.
- `docs/audit/SECURITY_STATUS.md` and `SECURITY.md` — security limitations and reporting policy.
- `docs/QWEN_CODER_PROMPT.md` — implementation handoff for Qwen Coder.

## Build and test (Linux / WSL2)

For the complete steps, supported-host notes, troubleshooting, and safety limits, follow the [Build and Boot Guide](docs/BUILD_AND_BOOT_GUIDE.md).

Install the nightly toolchain and targets:

```sh
rustup toolchain install nightly-2026-10-27 --profile minimal --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
sudo apt-get update
sudo apt-get install -y qemu-system-x86 ovmf
```

From the workspace directory:

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

CI runs the full QEMU/OVMF matrix and requires both the firmware splash message and `KEEFEOS-KERNEL-BOOT-OK`. Image generation alone does not establish boot success.

## Compatibility and security

The currently verified matrix is limited to x86-64 QEMU/OVMF at 256 MiB, 512 MiB, and 1 GiB RAM. Physical x86-64 devices, other hypervisors/firmware, ARM64, RISC-V, phones, consoles, and other boards are not supported or verified. See [the compatibility matrix](docs/COMPATIBILITY.md).

KeefeOS is not a security boundary. Secure-boot signature verification, trusted randomness, authenticated encryption, real capability verification, runtime vulnerability scanning, and attack detection remain unimplemented. Their APIs fail closed or report incomplete status.

Read [SECURITY.md](SECURITY.md), [the security status](docs/audit/SECURITY_STATUS.md), and [the initial codebase audit](docs/audit/INITIAL_CODEBASE_AUDIT.md).

## Architecture and planned direction

The current implementation is an experimental x86-64 kernel/UEFI boot path. The multi-device system and AI-agent experience are long-term goals, not working features yet.

- [Architecture guide](docs/ARCHITECTURE.md) — current crates, trust boundaries, and target layering.
- [Staged roadmap](docs/ROADMAP.md) — acceptance gates for kernel safety, user space, drivers, portability, and cross-device support.
- [AI agent security model](docs/AI_AGENT_SECURITY_MODEL.md) — intended MCP/API/provider design, least-privilege tools, credential handling, and human approvals.

**Design rule:** the LLM/agent runtime, API keys, MCP clients, and tool executors belong in isolated user space after process isolation and IPC enforcement exist. They must not run as privileged kernel code. Cross-device support must be proven separately for each architecture, firmware, and device configuration.

The [NEXUS design](NEXUS_OS_DESIGN.md), [roadmap](NEXUS_OS_ROADMAP.md), and [integration proposal](NEXUS_OS_INTEGRATION.md) are aspirational documents; they are not implementation evidence.

## Community and contribution

- [CONTRIBUTING.md](CONTRIBUTING.md) — development workflow, safety expectations, testing, and pull-request checklist.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — community behavior and reporting.
- [CONTRIBUTORS.md](CONTRIBUTORS.md) — contributor recognition policy.
- [SUPPORT.md](SUPPORT.md) — where to ask for help and what evidence to include.
- [SECURITY.md](SECURITY.md) — responsible vulnerability reporting.

Prioritize boot correctness, memory safety, regression tests, and evidence-backed implementation claims over speculative feature claims. Use the issue templates to report reproducible bugs or propose scoped improvements.

## License and copyright

KeefeOS is dual-licensed under **MIT OR Apache-2.0**, matching the Rust workspace metadata. You may choose either license; see [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE). Third-party components may have separate terms. Contributors retain their own copyright in their contributions; do not submit code or assets unless you have the right to license them under the project's stated terms.
