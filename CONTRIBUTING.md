# Contributing to KeefeOS

Thank you for helping improve KeefeOS. The project is an experimental x86-64 Rust OS prototype. Contributions should make behavior more correct and testable, not merely increase the number of claimed features.

## Before you start

1. Read the root README, the current audit in docs/audit, and the relevant code and architecture notes.
2. Check the current branch, commit, and working tree before editing. Preserve unrelated user changes.
3. Open an issue or discuss a large architectural change before investing in it. Small, well-scoped bug fixes can proceed directly.
4. Do not treat any NEXUS roadmap item or AI/security module name as proof that the feature works.

## Development environment

The currently documented build/image workflow is aimed at Linux or WSL2. Install the pinned nightly toolchain, Rust source and LLVM tools, plus the x86-64 bare-metal and UEFI targets. On Ubuntu/Debian, QEMU and OVMF are used for emulation.

~~~sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
cd KeefeOS
cargo +nightly fmt --all -- --check
cargo +nightly check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
~~~

These commands describe the intended checks, not guaranteed passing results. Consult the current Actions run for the exact commit and report any failure accurately. The currently documented CI evidence verifies an early-boot milestone under x86-64 QEMU + OVMF at three RAM sizes; it does not verify a complete OS or physical hardware. Image creation alone is not a boot test. See the [compatibility matrix](docs/COMPATIBILITY.md) and [current OS status](docs/audit/OS_STATUS.md).

## Contribution process

- Keep changes focused and explain the root cause of each bug fix.
- Add regression tests for logic that can be tested without hardware.
- For architecture- or hardware-dependent changes, add a reproducible smoke-test plan and distinguish it from tests actually run.
- Update documentation whenever a public command, boot invariant, interface, or implementation status changes.
- Do not commit generated target output, disk images, private credentials, tokens, keys, or local configuration.
- Minimize unsafe Rust. Each unsafe block must have a clear safety rationale, valid lifetime/alignment/range assumptions, and tests where practical.
- Do not disable a failing lint or test merely to turn CI green.
- Do not claim secure boot, capability isolation, cryptographic protection, real-time scheduling, process migration, or AI-powered threat detection works unless the implementation and evidence support that exact claim.
- Do not run destructive disk commands or install this prototype on physical hardware as a test. Use an isolated VM and disposable images.

## Git and branch policy

Use a short-lived topic branch when a change needs review, open a pull request targeting main, and delete the topic branch after a verified merge. The repository owner intends main to be the only persistent remote branch once the existing branch consolidation is complete. Never force-push, rewrite shared history, or delete a branch before reviewing its unique commits and confirming their disposition.

If repository branch protections require a pull request or approval, follow those rules rather than bypassing them.

## Pull request checklist

- [ ] Explain the defect or need and the chosen solution.
- [ ] List changed files and any architecture/API impact.
- [ ] Include exact commands and results for formatting, builds, tests, and emulation.
- [ ] Mark unavailable or unrun tests as NOT TESTED or BLOCKED.
- [ ] Update audit/status docs if implementation claims change.
- [ ] Review the final diff for unrelated edits, generated files, secrets, and false claims.
- [ ] Confirm the applicable CI run has finished before saying the change is verified.

## Areas that most need help

1. A correct ELF-to-kernel boot handoff or an explicitly selected compatible boot protocol.
2. A versioned, validated UEFI memory-map handoff and correct physical/virtual address handling.
3. Host-testable tests for frame allocation, map parsing, paging arithmetic, and overflow handling.
4. A scheduler/context-switch design with well-defined ABI, ownership, stack, interrupt, and lock invariants.
5. Reproducible CI and accurate documentation of what is implemented versus planned.

## Licensing

The workspace metadata declares MIT OR Apache-2.0. The repository includes the corresponding license texts in LICENSE-MIT and LICENSE-APACHE; the root LICENSE explains the choice. By submitting a contribution, you confirm that you have the right to submit it and agree that the contribution is offered under the project's stated dual-license terms, unless you explicitly state otherwise. Do not submit third-party material without verifying its license and attribution requirements. This guidance is not legal advice.
