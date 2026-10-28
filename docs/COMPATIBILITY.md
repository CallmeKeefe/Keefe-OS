# KeefeOS hardware and platform compatibility

**Status: early boot and UEFI splash verified in three QEMU RAM profiles; not an end-user installable OS.**

The latest completed passing run confirmed from GitHub records is [run 40698180312](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40698180312) on PR-head commit `9cfbbff013765ca88b3a999471ab2248787b1308`. It passed 22 boot-protocol tests, 51 kernel unit tests, dependency auditing, both target builds, packaging, and QEMU/OVMF early boot at 256 MiB, 512 MiB, and 1 GiB RAM. OVMF rendered the splash through GOP BLT and the kernel emitted `KEEFEOS-KERNEL-BOOT-OK` for each profile. This is evidence for that exact commit only; later main-branch code `c27a616c7330702c65213f185a0dec41810a6b4d` and subsequent workflow/doc changes still need confirmed runs.

| Platform / firmware | Current status | Evidence / limitation |
|---|---|---|
| x86-64 QEMU + OVMF, 256 MiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40698180312. |
| x86-64 QEMU + OVMF, 512 MiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40698180312. |
| x86-64 QEMU + OVMF, 1 GiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40698180312. |
| Kernel direct framebuffer console | **DISABLED / FAIL-CLOSED** | Direct writes remain disabled until target-specific page-table mapping and MMIO cache attributes are proven; firmware GOP BLT draws the startup splash before `ExitBootServices`. |
| Physical x86-64 PC with UEFI | NOT VERIFIED / NOT SUPPORTED FOR INSTALLATION | ACPI, PCI/device enumeration, storage/input/network drivers, power handling, and firmware variation need work. |
| Other UEFI firmware implementations | NOT VERIFIED | No vendor firmware matrix. |
| VirtualBox / VMware / Hyper-V | NOT VERIFIED | No smoke-test evidence. |
| Legacy BIOS | NOT SUPPORTED | No BIOS boot stage or image format. |
| ARM64 / AArch64 | NOT SUPPORTED | No AArch64 target, boot protocol, exception vectors, linker, or drivers. |
| RISC-V | NOT SUPPORTED | No RISC-V target, trap handling, linker, or boot protocol. |
| Apple Silicon Macs | NOT SUPPORTED as native boot targets | ARM64 and platform boot requirements are absent. |
| Android/iOS phones and tablets | NOT SUPPORTED | No mobile SoC, touch/display stack, power management, or driver support. |
| Consoles, embedded boards, SBCs | NOT SUPPORTED BY DEFAULT | Each needs a board-specific boot path, memory map, interrupt controller, and drivers. |

A passing result for the latest main revision has not been confirmed in this audit. Check the [current Actions runs](https://github.com/CallmeKeefe/Keefe-OS/actions) before treating any RAM profile as verified for newer commits.

## Next compatibility milestones

1. Investigate the GOP memory-map gap and design a safe MMIO/page-table mapping before enabling persistent kernel framebuffer writes.
2. Remove the fixed load-address limitation with a properly tested relocatable kernel, or keep the fixed layout explicitly constrained.
3. Add QEMU device/firmware variations and repeated boot tests.
4. Implement ACPI, PCI discovery, and selected virtual storage/input/network drivers only after memory and boot contracts stabilize.
5. Test selected physical hardware only with safe non-destructive procedures and a recovery plan.
6. Build independent AArch64/RISC-V architecture backends and CI; do not claim support until they build and pass boot tests.

## Safety

Do not install this prototype on physical hardware or disks with valuable data. There is no end-user installer, upgrade path, recovery environment, or production configuration.
