# KeefeOS hardware and platform compatibility

**Status: early boot and UEFI splash verified in three QEMU RAM profiles; not an end-user installable OS.**

Evidence: [GitHub Actions run 40201976679](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201976679), commit `df97691cdecd26ff3a099b981b542817088683b1`. The workflow passed 22 boot-protocol tests, 31 kernel tests, both target builds, image packaging, and the serial boot/splash smoke test at 256 MiB, 512 MiB, and 1 GiB RAM. OVMF rendered the splash through GOP BLT before ExitBootServices, and the kernel emitted `KEEFEOS-KERNEL-BOOT-OK` in each profile.

| Platform / firmware | Current status | Evidence / limitation |
|---|---|---|
| x86-64 QEMU + OVMF, 256 MiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40201976679. |
| x86-64 QEMU + OVMF, 512 MiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40201976679. |
| x86-64 QEMU + OVMF, 1 GiB RAM | **EARLY BOOT VERIFIED** | Kernel marker and firmware GOP splash passed in run 40201976679. |
| Kernel direct framebuffer console | **FAIL-CLOSED / NOT ENABLED** | The final UEFI memory map has a gap over the GOP range. Direct kernel writes are refused. |
| Physical x86-64 PC with UEFI | NOT VERIFIED / NOT SUPPORTED FOR INSTALLATION | ACPI, PCI/device enumeration, storage/input/network drivers, power handling, and firmware variation need work. |
| Other UEFI firmware implementations | NOT VERIFIED | No vendor firmware matrix. |
| VirtualBox / VMware / Hyper-V | NOT VERIFIED | No smoke-test evidence. |
| Legacy BIOS | NOT SUPPORTED | No BIOS boot stage or image format. |
| ARM64 / AArch64 | NOT SUPPORTED | No AArch64 target, boot protocol, exception vectors, linker, or drivers. |
| RISC-V | NOT SUPPORTED | No RISC-V target, trap handling, linker, or boot protocol. |
| Apple Silicon Macs | NOT SUPPORTED as native boot targets | ARM64 and platform boot requirements are absent. |
| Android/iOS phones and tablets | NOT SUPPORTED | No mobile SoC, touch/display stack, power management, or driver support. |
| Consoles, embedded boards, SBCs | NOT SUPPORTED BY DEFAULT | Each needs a board-specific boot path, memory map, interrupt controller, and drivers. |

## Next compatibility milestones

1. Investigate the GOP memory-map gap and design a safe MMIO/page-table mapping before enabling persistent kernel framebuffer writes.
2. Remove the fixed load-address limitation with a properly tested relocatable kernel, or keep the fixed layout explicitly constrained.
3. Add QEMU device/firmware variations and repeated boot tests.
4. Implement ACPI, PCI discovery, and selected virtual storage/input/network drivers only after memory and boot contracts stabilize.
5. Test selected physical hardware only with safe non-destructive procedures and a recovery plan.
6. Build independent AArch64/RISC-V architecture backends and CI; do not claim support until they build and pass boot tests.

## Safety

Do not install this prototype on physical hardware or disks with valuable data. There is no end-user installer, upgrade path, recovery environment, or production configuration.
