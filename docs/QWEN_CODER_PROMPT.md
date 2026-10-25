# Qwen Coder handoff — KeefeOS reliability, graphics, security and compatibility

Repository: https://github.com/CallmeKeefe/Keefe-OS
Canonical branch: main

## Current verified milestone

At commit `df97691cdecd26ff3a099b981b542817088683b1`, [GitHub Actions run 40201976679](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201976679) passed 22 boot-protocol tests, 31 kernel unit tests, the host helper check, both builds, image packaging, and the QEMU/OVMF smoke-test matrix with 256 MiB, 512 MiB, and 1 GiB RAM. In all three, the serial output included the UEFI GOP splash message and `KEEFEOS-KERNEL-BOOT-OK`.

The UEFI splash renders readable pixel text using firmware GOP BLT before ExitBootServices. The direct kernel framebuffer console stays disabled because the final memory map has a gap over the GOP range at `0x80000000`. Do not weaken that protection or claim the kernel framebuffer works until a valid MMIO/page-table mapping is designed and tested.

## Mission

Continue actual implementation, testing, and safety improvements. Inspect live HEAD, status, refs, and CI before editing. Do not repeat already completed work or turn unverified goals into claims.

## Non-negotiable rules

- Preserve user changes; inspect Git status, HEAD, refs, tags and workflow state first.
- Do not force-push, rewrite shared history, or delete branches before reviewing and safely disposing of unique commits.
- Never claim a build/test/boot/security control works without evidence.
- Never commit secrets or generated artifacts.
- No physical disk/partition/dual-boot commands; use disposable QEMU images.
- Current supported scope is x86-64 QEMU/OVMF at the tested 256/512/1024 MiB profiles. Do not claim ARM64, RISC-V, phones or universal PC compatibility.
- Keep crypto/capability paths fail-closed until real primitives and trust roots are integrated.

## Highest-priority next tasks

### 1. Persistent graphics memory contract
- Investigate why the final UEFI memory map has a gap across GOP framebuffer memory at `0x80000000`.
- Preserve the current serial fallback and strict range validation.
- Choose and document a correct mapping strategy for the framebuffer (page attributes/cache policy, MMIO semantics, lifetime, and physical/virtual address ownership).
- If required, establish an explicit mapping before writing and validate the mapping lifetime. Never write raw framebuffer memory based on GOP metadata alone.
- Add tests for frame-buffer stride, clipping, RGB/BGR packing, address overflow and mapping failure.
- Keep firmware BLT splash verified separately from kernel framebuffer rendering.

### 2. Load-address and memory safety
The kernel remains linked into a fixed 32–64 MiB x86-64 window. OVMF boot succeeds at 256/512/1024 MiB QEMU RAM profiles, but relocatability is not implemented. Prefer a correctly built ET_DYN/PIE kernel with a well-tested relocation path, or keep the supported window constrained and fail cleanly if unavailable. Do not retry random physical addresses for non-relocatable code.

### 3. Paging and allocator
- Review BootInfo normalization/reservations for kernel, BootInfo, stack, page tables and bootloader allocations.
- Expand memory-map and allocator tests for gaps, overflow, unaligned ranges, exhaustion, and varied QEMU memory.
- Reconcile or isolate duplicate paging modules. Do not claim higher-half mapping or user/kernel isolation before tested page tables are active.
- Preserve the early bump heap's bounded behavior; add reclamation only with complete invariants and tests.

### 4. Security
- Secure boot, cryptography, trusted random generation, signatures, capability authorization, vulnerability scanning and attack detection remain NOT IMPLEMENTED.
- Review unsafe code around firmware buffers, physical memory, page tables, interrupts and framebuffer.
- Add negative tests for malformed BootInfo and unverified grants.
- No fabricated security score or “clean scan” from empty placeholder results.

### 5. CI/reliability
- Keep 22 protocol tests, 31 kernel tests, target builds, packaging and the 3-profile QEMU boot/splash matrix as checks.
- Keep one disposable OVMF variable store per emulator run/profile and preserve per-profile serial logs.
- Test release profile, and classify/fix warnings without blanket suppression.
- Continue other virtual device/firmware variation only when results can be diagnosed and reproduced.

### 6. Docs and branches
- Keep README, COMPATIBILITY, OS_STATUS, SECURITY_STATUS, setup, installation/testing and this prompt synchronized to the newest verified CI.
- Do not describe the project as installable on physical hardware.
- Complete branch review/consolidation with explicit dispositions; remove non-main branches only when all unique changes have been reviewed and remote state is confirmed.

## Final response format

Report current HEAD, changed paths/commit IDs, all test results, current CI URL, serial evidence for each memory profile, graphics behavior (firmware splash vs kernel console), compiler warning count/status, hardware/architecture limits, and remaining blockers. Use PASS/FAIL/BLOCKED/NOT TESTED accurately. Never claim physical-installability, production security, or universal compatibility without evidence.
