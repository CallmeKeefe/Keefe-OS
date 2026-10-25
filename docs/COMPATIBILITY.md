# KeefeOS hardware and platform compatibility

**Status: early boot and firmware-rendered splash verified in one emulator configuration; not an end-user installable OS.**

The latest evidence is [GitHub Actions run 40201424209](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40201424209), commit `98f1830908c57861910df866a51dfafa138b0d04`. It passed 22 boot-protocol tests, 31 kernel tests, both target builds, image packaging, and a QEMU/OVMF serial boot smoke test. OVMF accepted the GOP BLT operations used to draw a readable splash before ExitBootServices. The kernel reached `KEEFEOS-KERNEL-BOOT-OK`.

| Platform / firmware | Current status | Evidence / limitation |
|---|---|---|
| x86-64 QEMU + OVMF UEFI | **EARLY BOOT + UEFI SPLASH VERIFIED** | BootInfo validation and kernel idle-path marker observed. GOP firmware BLT drew the pre-boot splash. Fixed 32–64 MiB load window; one virtual hardware profile only. |
| Kernel direct framebuffer console | **FAIL-CLOSED / NOT ENABLED** | The final UEFI memory map has a gap over the GOP framebuffer range. Kernel refuses direct writes rather than assume that an untracked range is mapped safely. |
| Physical x86-64 PC with UEFI | NOT VERIFIED / NOT SUPPORTED FOR INSTALLATION | ACPI, PCI/device enumeration, storage/input/network drivers, platform power handling and diverse firmware need work. |
| Other UEFI firmware implementations | NOT VERIFIED | No test matrix across vendor firmware or differing memory maps. |
| VirtualBox / VMware / Hyper-V | NOT VERIFIED | No serial smoke-test evidence yet. |
| Legacy BIOS | NOT SUPPORTED | No BIOS boot stage or BIOS image format. |
| ARM64 / AArch64 | NOT SUPPORTED | No AArch64 kernel target, boot protocol, exception vectors, linker or drivers. |
| RISC-V | NOT SUPPORTED | No RISC-V target, trap handling, linker or boot protocol. |
| Apple Silicon Macs | NOT SUPPORTED as a native boot target | ARM64 and platform boot requirements are not implemented. |
| Android/iOS phones and tablets | NOT SUPPORTED | No mobile SoC support, touch/display stack, power management or drivers. |
| Consoles, embedded boards, SBCs | NOT SUPPORTED BY DEFAULT | Each needs a board-specific boot path, memory map, interrupt controller and drivers. |

## Next compatibility milestones

1. Define and test an explicit MMIO mapping contract for the GOP framebuffer; do not weaken the reserved-range check merely to enable drawing.
2. Remove the fixed load-address limitation by implementing a relocatable kernel and verified relocation handling, or keep the fixed layout explicitly constrained.
3. Test memory-map parsing and allocator behavior across varied QEMU memory sizes and virtual device profiles.
4. Add ACPI parsing, PCI enumeration, and selected virtual storage/input/network drivers.
5. Test alternate QEMU configurations, then selected physical hardware with recovery and non-destructive test procedures.
6. Build separate AArch64/RISC-V architecture backends with boot, exception, paging, atomics and CI; do not advertise compatibility until tests pass.

## Safety

Do not install this prototype on physical hardware or on disks with valuable data. There is no supported end-user installer, upgrade path, recovery environment or production configuration.
