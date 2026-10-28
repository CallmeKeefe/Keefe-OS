# CPU exceptions and IDT: implementation notes

**Status: experimental x86-64 kernel groundwork. This is not a claim that the full interrupt subsystem or user-mode isolation is implemented.**

The current code sets up GDT/TSS and an IDT with CPU exception handlers. The double-fault path is configured to use an IST stack. The current CI boot test confirms that the kernel reaches its boot marker under the tested QEMU/OVMF profiles; it does not exercise every exception path or establish correctness under physical hardware.

## Current boundaries

- CPU exception handlers exist for the implemented vectors and provide serial diagnostics.
- Double-fault stack setup exists, but should be reviewed with dedicated fault-injection tests before being treated as robust.
- Hardware IRQ routing, PIC/APIC initialization, EOI handling, and general device interrupt delivery are not implemented as a complete subsystem.
- User-mode execution, process ownership, and per-process fault termination are not implemented. A page-fault diagnostic mentioning Ring 3 behavior must not be treated as a process isolation implementation.
- Detailed logs are for development and can expose memory addresses; do not use them as a production audit facility.

## Validation expectations

For interrupt-related changes:
1. Preserve the current build and QEMU/OVMF matrix.
2. Add deterministic host tests for descriptor construction and pure helpers where possible.
3. Add isolated QEMU fault-injection tests before making claims about runtime exception behavior.
4. Never enable external interrupts before handlers, stack state, controller configuration, and acknowledgement semantics are correct.
5. Keep safety comments tied to the actual ABI, IST index, lifetime, and mapping assumptions.

## Authoritative status

- [OS status](../../docs/audit/OS_STATUS.md)
- [Security status](../../docs/audit/SECURITY_STATUS.md)
- [Architecture guide](../../docs/ARCHITECTURE.md)
- [Build and boot guide](../../docs/BUILD_AND_BOOT_GUIDE.md)

This page is retained as a subsystem note; repository-root documentation and current CI results are authoritative.
