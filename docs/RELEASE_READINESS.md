# KeefeOS release and installation readiness gates

**Current decision: experimental VM-only prototype. Not ready for physical installation or general-purpose use.**

This checklist prevents a successful early-boot demo from being mistaken for an installable operating system. Change the status only when the evidence below exists for the exact release commit.

## Supported use today

- Build and test on a documented Linux/WSL2 host.
- Boot the x86-64 UEFI image in an isolated QEMU + OVMF virtual machine.
- Use disposable VM disks and snapshots.
- Contribute code and report reproducible emulator results.

Do not write the prototype image to a physical USB drive or disk, dual-boot it, or use it with data that must be preserved.

## Gate A — publish a reproducible VM demo

- [ ] Latest main CI completed successfully on the exact commit being shared.
- [ ] Boot-protocol and kernel tests pass on that commit.
- [ ] RustSec audit and all required Clippy/build steps pass.
- [ ] QEMU/OVMF matrix passes and serial logs show both expected markers.
- [ ] The generated image artifact is attached to that exact successful run.
- [ ] Image SHA-256 is published and can be independently recomputed.
- [ ] Instructions name the exact commit, host dependencies, QEMU command, expected output, timeout behavior, and cleanup procedure.
- [ ] No test step writes to a host block device or real disk.
- [ ] The release notes state the limits of the tested environment and all known failures.

## Gate B — consider physical installation

**Every item is required; passing Gate A is not enough.**

- [ ] A reviewed installer that enumerates and displays target disks, requires explicit confirmation, refuses ambiguous targets, and never silently formats a disk.
- [ ] A tested recovery/rescue path and documented uninstall procedure.
- [ ] An upgrade and rollback strategy, including interruption/power-loss handling.
- [ ] A hardware compatibility matrix covering firmware vendors, memory layouts, CPUs, and relevant devices.
- [ ] A robust boot protocol and validated memory-map ownership/lifetime model.
- [ ] Correct page-table and MMIO mapping rules; no direct framebuffer writes until mapping and cache attributes are proven.
- [ ] Storage, input, networking, interrupt routing, and shutdown/reboot support appropriate to the advertised scope.
- [ ] A threat model, security review, and tested privilege/isolation boundaries.
- [ ] Reproducible builds, signed release artifacts, provenance, and checksums.
- [ ] Independent tests on disposable hardware with no user data at risk.
- [ ] Explicit release approval and a clear list of unsupported hardware.

## Contributor checklist

- Use a topic branch and pull request.
- Explain the root cause and add regression coverage where possible.
- Include exact commands and results; mark unrun checks as NOT TESTED.
- Do not suppress failing checks or describe placeholder interfaces as working security features.
- Keep changes small enough for independent review.
- Never submit secrets, tokens, private keys, or generated disk images.

## Current known blockers

- No supported physical installer, recovery environment, or upgrade path.
- General-purpose storage, USB, input, networking, and power-management drivers are absent.
- Paging/address-space lifecycle and isolation are experimental.
- Direct kernel framebuffer mapping is disabled pending mapping and cache-attribute proof.
- Secure boot, production cryptography, and real attack/vulnerability detection are not implemented.
- Only the x86-64 QEMU + OVMF early-boot path has meaningful automated evidence; physical hardware and other architectures are not supported.

Update this file only with evidence from the exact commit being evaluated. A plan, successful image build, or passing older commit does not satisfy a gate.
