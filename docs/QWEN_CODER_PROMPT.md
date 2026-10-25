# Qwen Coder handoff — KeefeOS reliability, graphics, security and compatibility

Repository: https://github.com/CallmeKeefe/Keefe-OS
Canonical branch: main

## Current verified milestone

At commit `2683ea911db964a68f86c615372fc13171bf3c4e`, [GitHub Actions run 40199251364](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40199251364) passed:
- host-side xtask check;
- BootInfo and ELF parser tests;
- host-side kernel library tests;
- kernel ELF build;
- UEFI loader build;
- FAT image assembly;
- QEMU/OVMF serial smoke test, observing `KEEFEOS-KERNEL-BOOT-OK`.

The firmware loaded the image whose physical range was logged as `0x2000000..0x2815000`, exited boot services, passed the BootInfo handoff, and the kernel reached its early idle path. This is one x86-64 QEMU + OVMF test. It does not prove physical hardware support, a production OS, secure boot, robust long-running runtime behavior, or universal compatibility.

## Mission

Continue actual implementation and verification. Inspect the live HEAD and CI status before edits; the repo may have moved. Do not simply repeat completed work or replace evidence-bearing reports with generic prose.

## Non-negotiable rules

- Preserve user changes; inspect `git status`, HEAD, refs, tags and workflow state first.
- Do not force-push, rewrite shared history, or delete branches before review and verified merge.
- Do not claim a build/test/boot/security control works without evidence.
- Never reveal or commit secrets.
- No physical disk/partition/dual-boot commands; use disposable QEMU images.
- Current supported target is x86-64 only. Do not claim ARM64, RISC-V, phones or all PCs are compatible.
- Keep cryptography and capability paths fail-closed until real primitives and trust roots are integrated.

## Highest-priority next tasks

### 1. Remove load-address fragility
The current kernel is linked to a fixed physical window 32–64 MiB. The loader reserves the exact ET_EXEC span. This passed in QEMU/OVMF but is not portable across memory maps. Prefer an ET_DYN/PIE kernel plus a well-tested relocation path if it can be implemented soundly; otherwise document and validate the exact supported layout and fail with useful diagnostics. Never merely retry random addresses while executing non-relocatable code.

### 2. Build a graphics path
- Query UEFI GOP before ExitBootServices.
- Record framebuffer base, width, height, stride and supported pixel format in BootInfo.
- Validate buffer size, stride, pixel format, address arithmetic and mapping before writing.
- Add a minimal framebuffer renderer or splash with serial fallback; do not assume VGA text memory at 0xB8000.
- Add a framebuffer unit test for pixel packing and clipping, and retain the serial boot marker in CI.
- Label unsupported GOP pixel formats accurately rather than writing unsafe color values.

### 3. Memory and paging
- Review BootInfo descriptor normalization and reserve all bootloader/kernel/BootInfo/stack/page-table ranges correctly.
- Continue tests for overflow, invalid descriptors, holes, unaligned ranges and allocator exhaustion.
- Reconcile or remove duplicated experimental paging implementations. Do not enable the higher-half model before the active page-table and linker contract is correct.
- Add QEMU memory-size matrix tests when practical.
- Preserve safe semantics of the early bump heap; implement reclaiming allocation only with proper invariants and tests.

### 4. Security
- Keep cryptography, capability authorization, secure boot signatures, scanner and attack detector marked NOT IMPLEMENTED while they are placeholders.
- Review every unsafe block around physical memory, firmware buffers, page tables, interrupts and framebuffer writes.
- Add negative tests for malformed boot data and unverified capabilities.
- Do not fabricate security scores; empty scanner results are not a clean audit.

### 5. Reliability, warnings and CI
- Current CI passes the cited run but kernel build/test emits many compiler warnings (the kernel library test build previously reported roughly 190 warnings). Classify and fix real warnings; avoid mass-suppressions.
- Keep protocol/kernel tests, kernel/UEFI builds, image assembly, and QEMU serial boot smoke test as required checks.
- Test release-profile build, because build-script and artifact paths can differ.
- Ensure QEMU exits/fails clearly if no marker; preserve serial artifacts.
- Distinguish successful early boot from working processes, scheduler, graphics, drivers, installer or production use.

### 6. Documentation and branch policy
- Keep README, installation/testing guide, compatibility matrix, security status, and audit docs up to date with the latest actual CI run.
- The current installation guide must not suggest physical installation, destructive disk operations, or unsupported host/architecture compatibility.
- Continue branch inventory/consolidation. Document every branch's unique changes. Only clean non-main remote branches after unique work has a disposition and main is verified; respect protections and actual permissions.

## Required final report

Include exact current HEAD, changed files/commit IDs, commands/tests and PASS/FAIL/BLOCKED/NOT TESTED outcomes, current Actions URL, serial boot evidence, remaining warnings and hardware limitations. Do not claim clean build, production security, complete device support, or branch deletion unless verified.
