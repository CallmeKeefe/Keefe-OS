# KeefeOS hardware and platform compatibility

**Status: early boot verified in one emulator configuration; not an end-user installable OS.**

The latest verified run is [GitHub Actions run 40199251364](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40199251364), commit `2683ea911db964a68f86c615372fc13171bf3c4e`. It built the kernel and UEFI loader, packaged an image, and observed `KEEFEOS-KERNEL-BOOT-OK` over serial in QEMU/OVMF.

| Platform / firmware | Current status | Evidence / limitation |
|---|---|---|
| x86-64 QEMU + OVMF UEFI | **EARLY BOOT VERIFIED** | BootInfo validation and kernel idle-path marker observed in the cited CI run. Fixed 32–64 MiB load window; one virtual hardware profile only. |
| Physical x86-64 PC with UEFI | NOT VERIFIED / NOT SUPPORTED FOR INSTALLATION | ACPI, GOP graphics, PCI/device enumeration, storage/input/network drivers, platform power handling and diverse firmware need work. |
| Other UEFI firmware implementations | NOT VERIFIED | No test matrix across vendor firmware or differing memory maps. |
| VirtualBox / VMware / Hyper-V | NOT VERIFIED | No serial smoke-test evidence yet. |
| Legacy BIOS | NOT SUPPORTED | No BIOS boot stage or BIOS image format. |
| ARM64 / AArch64 | NOT SUPPORTED | No AArch64 kernel target, boot protocol, exception vectors, linker or drivers. |
| RISC-V | NOT SUPPORTED | No RISC-V target, trap handling, linker or boot protocol. |
| Apple Silicon Macs | NOT SUPPORTED as a native boot target | ARM64 and platform boot requirements are not implemented. |
| Android/iOS phones and tablets | NOT SUPPORTED | No mobile SoC support, touch/display stack, power management or drivers. |
| Consoles, embedded boards, SBCs | NOT SUPPORTED BY DEFAULT | Each needs a board-specific boot path, memory map, interrupt controller and drivers. |

## Next compatibility milestones

1. Remove the fixed load-address limitation by implementing a relocatable kernel and verified relocation handling, or explicitly constrain supported memory layouts.
2. Establish a stronger physical/virtual mapping contract and memory-map/allocator tests across varied QEMU memory sizes.
3. Query UEFI GOP and initialize a framebuffer console through the BootInfo contract.
4. Add ACPI parsing, PCI enumeration, and selected virtual storage/input/network drivers.
5. Test alternate QEMU device/memory profiles, then selected physical hardware.
6. Build separate AArch64/RISC-V architecture backends with boot, exception, paging, atomics and CI; do not advertise compatibility until tests pass.

## Safety

Do not install this prototype on physical hardware or on disks with valuable data. There is no supported end-user installer, upgrade path, recovery environment or production configuration.
