# KeefeOS hardware and platform compatibility

**Compatibility status: prototype; no end-user install target is currently verified.**

The current kernel uses x86-64 CPU instructions, an x86-specific UEFI loader target, x86 interrupt descriptors, a COM1 serial console, and a linker script for x86-64. Compiling the workspace is not the same as booting it.

| Platform / firmware | Current status | What is known |
|---|---|---|
| x86-64 QEMU + OVMF UEFI | BUILD/PACKAGE VERIFIED; BOOT TEST FAILED AT FIXED-RANGE ALLOCATION | The loader validates the ELF and tries to reserve its static 1–33 MiB load range, but OVMF returned EFI_NOT_FOUND. The load window is being moved to 32–64 MiB; verify the next CI run. |
| Modern x86-64 PC with UEFI | NOT SUPPORTED FOR INSTALLATION YET | Firmware/ACPI/memory-map handoff, GOP output, device discovery, and boot sequence are not complete. |
| x86-64 with legacy BIOS | NOT SUPPORTED | No BIOS boot stage or BIOS image format. |
| ARM64 / AArch64 devices | NOT SUPPORTED | No AArch64 kernel target, exception vectors, linker, boot protocol, or device drivers. |
| RISC-V systems | NOT SUPPORTED | No RISC-V target, trap handling, linker, or boot protocol. |
| Apple Silicon Macs | NOT SUPPORTED | ARM64 hardware and Apple platform boot requirements are not implemented. |
| Android/iOS phones and tablets | NOT SUPPORTED | No mobile SoC support, touch/display stack, power management, or driver platform. |
| Consoles, embedded boards, SBCs | NOT SUPPORTED BY DEFAULT | Each requires a board-specific boot path, memory map, interrupt controller, and drivers. |
| VirtualBox / VMware / Hyper-V | NOT VERIFIED | Could be assessed after the kernel has a valid UEFI-to-kernel handoff; current build alone does not establish compatibility. |

## Recommended compatibility milestones

1. **M1 — Deterministic x86-64 UEFI boot:** valid ELF64 segment loading, validated BootInfo, memory reservations, and serial milestone in QEMU/OVMF.
2. **M2 — Robust memory and exceptions:** tested allocator invariants, page-table contract, IDT/GDT/TSS, and fault diagnostics.
3. **M3 — Firmware graphics and input:** discover and initialize UEFI GOP from BootInfo; never assume legacy VGA text memory exists.
4. **M4 — Hardware support:** PCI enumeration, storage/USB/network/display drivers, power management, ACPI, and compatibility tests on selected hardware.
5. **M5 — Multi-architecture:** create separate architecture backends for AArch64 and RISC-V with per-architecture boot, exception, paging, atomics, and CI. Do not advertise these targets until they build and pass emulator smoke tests.

## Safety

Do not install this prototype on physical hardware or disks containing valuable data. No installer, upgrade path, recovery environment, or supported production configuration is provided.
