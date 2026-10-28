# Research note: portable KeefeOS with personal AI agents

**Reviewed:** 2026-10-28  
**Audience:** contributors planning the architecture, boot, security, and AI roadmap  
**Status:** engineering research and recommendations; not a claim that the proposed features are implemented.

## Executive conclusion

KeefeOS should be built as a **small trusted kernel plus architecture backends and user-space services**. The kernel should enforce memory ownership, process isolation, IPC, and permission boundaries. An agent runtime should run outside the kernel and interact with the OS through typed, policy-controlled interfaces.

The current project is an experimental x86-64 UEFI loader/kernel path with QEMU/OVMF early-boot evidence. Its current test matrix does not establish physical device support, complete page-table isolation, user-space processes, a driver ecosystem, networking, a secure credential store, or an operating AI agent. Those are substantial prerequisites, not polish tasks.

"All devices" is not a single compile target. The sensible long-term target is one project with shared protocol/service/UI code where possible, while each supported architecture and hardware family has a separate boot path, CPU/MMU backend, firmware integration, drivers, and reproducible acceptance tests.

## 1. Current state observed in the repository

| Area | Current state | Engineering consequence |
|---|---|---|
| Boot | x86-64 UEFI loader; ELF64 parser accepts a narrow fixed load window | Keep the current QEMU/OVMF path as the first support contract. Do not claim relocatable images or generic firmware compatibility. |
| Firmware ownership | Loader transfers a normalized memory map in BootInfo and exits boot services | The handoff and memory ownership must stay precise; boot services are no longer available after a successful ExitBootServices call. |
| Physical memory | Bitmap frame allocator has regression tests and a separate allocatable bitmap | Reserved/non-usable ranges and the kernel image must never be returned to the free pool. Deallocation still depends on caller ownership/lifetime guarantees and is therefore unsafe. |
| Virtual memory | Default paging initialization is deliberately disabled; the hand-written paging prototypes are behind `experimental-paging` | Do not treat mapping flags or arithmetic helpers as implemented process isolation. Consolidate to one implementation before creating user processes. |
| CPU/interrupts | x86-64 GDT/TSS/exception-table groundwork exists; device IRQ routing and controller acknowledgement are incomplete | Do not enable general hardware interrupts until routing, EOI, stack, re-entrancy, and fault-injection tests are complete. |
| Devices/services | No general ACPI/PCI/storage/USB/network/input/display/power driver stack | Installer, recovery, update, and physical-device support are not ready. |
| Security | Several APIs fail closed or report NOT IMPLEMENTED; crypto, signature verification, trusted entropy, capability enforcement, and real scanning are incomplete | These stubs are useful honesty, not usable security features. Keep the OS out of sensitive workloads. |
| AI | `kernel/src/llm_agent` is a local command-string demo, not a connected LLM or system manager | Build agent runtime in user space only after a real syscall/IPC/capability boundary exists. |
| Branches | Many historical branches are far behind; three have unique commits reviewed as unsafe or unsupported | Preserve main as canonical. Review unique commits individually; do not merge generated branch contents wholesale. |

## 2. Firmware and boot handoff research

The UEFI specification says `ExitBootServices()` terminates boot services; after a successful handoff the OS loader must continue system operation and cannot continue relying on the firmware's boot services. The loader must use the current memory-map key during that transition and preserve resources that must remain owned by firmware/runtime services.

For KeefeOS this means:

1. Finish all allocations and firmware protocol access before exiting boot services.
2. Preserve a validated, versioned handoff structure with bounded descriptor count, alignment checks, overflow rejection, and semantic validation.
3. Reserve loader/kernel/BootInfo allocations correctly, and never treat unrecognized, runtime, MMIO, reserved, or overlapping descriptor ranges as ordinary RAM.
4. Keep the GOP framebuffer fail-closed unless the full address range and mapping/caching behavior are proven.
5. Add negative tests for malformed BootInfo, overflowing ranges, reserved/usable overlaps, truncated pointers, and failure to obtain a complete memory map.
6. Treat QEMU/OVMF RAM variants as one firmware/device configuration with varied RAM—not as evidence for broad physical-device coverage.

Reference: [UEFI Specification 2.11](https://uefi.org/specs/UEFI/2.11/), especially the overview and boot-services sections; [ExitBootServices overview](https://uefi.org/specs/UEFI/2.11/02_Overview.html).

## 3. Rust bare-metal implications

Rust's `x86_64-unknown-none` target is a freestanding target without `std` or a default allocator. The kernel must supply its allocator before relying on `alloc`, define a stable boot/linker contract, and explicitly manage code-generation assumptions. The target's Rust test suite is not run as a normal bare-metal executable; host-testable logic should be separated from hardware execution, with QEMU tests covering the boot/architecture boundary.

KeefeOS already has a good workspace split:
- `boot-protocol` for a shared handoff/ELF parser;
- `bootloader` for the UEFI application;
- `kernel` for bare-metal code;
- `xtask` for host build/image/test orchestration.

Keep this split and avoid reintroducing a second active paging implementation. Keep pure parsing/arithmetic/permission policy logic host-testable where feasible; keep unsafe hardware operations small and surrounded by precise safety contracts.

Reference: [Rust compiler platform support: x86_64-unknown-none](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html).

## 4. What "portable across devices" actually requires

### x86-64 UEFI PCs

Continue with QEMU + OVMF first. The next useful target is not "all PCs" but a named set of virtual devices and firmware configurations. Add ACPI parsing, PCI enumeration, timers/interrupt controllers, a device model, storage, input, display, and networking in a staged order. Every new device profile needs logs and a test that fails when the expected device/service is absent.

### ARM64 computers and boards

AArch64 is a separate backend, not an x86-64 build flag. It requires its own entry code, exception vectors, MMU/page-table logic, linker/script and memory model, interrupt controller, timers, cache maintenance, boot protocol, and target validation. Device discovery may use ACPI or a Device Tree depending on the platform/boot contract; Linux's ARM ACPI documentation explicitly discusses the relationship between these alternatives and UEFI table handoff.

Start with a named emulator configuration, then select a short list of physical boards only after serial output, memory map, timers/interrupts, storage, input and recovery have a reproducible test path.

Reference: [Linux kernel: ACPI on Arm systems](https://www.kernel.org/doc/html/latest/arch/arm64/arm-acpi.html).

### Phones and tablets

Do not promise a universal installable mobile OS yet. Per-device boot restrictions, vendor firmware/boot chains, GPU/display, touch, storage, Wi-Fi/cellular, audio, camera, sensors, suspend/resume, power management, secure update, and recovery all need device-specific work. An app running on Android/iOS is a possible companion/control surface, not the same as replacing that platform's OS.

A pragmatic product route is:
1. Keep the bare-metal kernel as its own experimental platform.
2. Build a cross-platform agent/UI client that can run on supported existing operating systems while the kernel matures.
3. Later add native KeefeOS targets one named architecture/device profile at a time.
4. Reuse typed protocols, tool adapters, settings schemas, and UI logic where platform boundaries allow, while not pretending drivers or kernel code are universally portable.

### Driver architecture

Define a common device/service contract with device-specific IDs and resources rather than one monolithic generic driver. Linux's device-model documentation is a useful reference for separating bus discovery, device description, driver matching, probe/remove, resource ownership and power callbacks. KeefeOS need not copy Linux wholesale, but it should have these concepts before broad hardware claims.

Reference: [Linux kernel device model](https://docs.kernel.org/driver-api/driver-model/overview.html).

## 5. Personal AI agent, LLM API and MCP design

### Proposed component boundaries

1. **Kernel:** process memory protection, IPC checks, scheduling, resource ownership, and permission enforcement.
2. **Agent service (future user space):** model/provider calls, planning, task state, and response rendering.
3. **Policy broker (future user space backed by kernel-enforced permissions):** checks principal, capabilities, target, risk, consent, rate/resource limits, and revocation.
4. **Tool adapters:** typed local operations, MCP clients, API providers, and optional remote agents.
5. **UI:** shows the plan, exact tool/action, destination/data sharing, risk, approval, cancellation, and outcome.

Do not put provider keys, an arbitrary shell, network client, or LLM inference loop in Ring 0. A prompt is not a permissions system. An LLM must not be able to authorize itself or bypass local checks.

### Safe rollout

**MVP 0 — no LLM required:** deterministic system status queries and read-only diagnostics.

**MVP 1 — read-only tools:** UI plus a typed local tool registry, with explicit per-tool grants and bounded arguments/results.

**MVP 2 — optional hosted/local model:** add a provider interface and an opt-in API key. Keep the core system usable without Internet, key, or model. Disclose which data leaves the device.

**MVP 3 — MCP/API tools:** allowlist servers and tools; validate JSON-RPC envelopes, schemas, response size, timeout, concurrency, and server identity. Treat all remote descriptions and tool results as untrusted content.

**MVP 4 — controlled writes:** present a diff/preview and require approval for file changes, sending data, installing software, or changing settings. Start with no arbitrary privileged shell and no destructive silent background actions.

**MVP 5 — more autonomous workflows:** only after enforced process isolation, permissions, sandboxing, audit, cancellation, secret management, reliable recovery, adversarial testing, and independent review. Risky operations should require approval based on action type, not on model confidence.

### MCP threat controls

- Confirm before connecting new MCP servers; identify publisher and endpoint.
- Re-prompt when server tool definitions/permissions materially change.
- Do not let web content or an LLM response install a server or register privileged tools.
- Treat approved-server responses as untrusted; remote tools can still be compromised.
- Authorize every tool call in trusted code after the model proposes it.
- Scope file access to specific roots and reject path traversal.
- Do not expose secrets in prompts, chat history, telemetry, shell arguments, or crash logs.
- Apply timeout, response-size, concurrency, cancellation and rate limits.
- Log decisions and results without logging secrets.
- Verify the actual postcondition before reporting that a task succeeded.

References: [OWASP MCP Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP_Security_Cheat_Sheet.html); [systemd credentials design](https://systemd.io/CREDENTIALS/) as an example of separating service credentials from the broader process environment. These are design references, not dependencies or features currently shipped by KeefeOS.

## 6. Security and release gates

Do not describe KeefeOS as secure or ready for sensitive data until the following have been implemented and independently reviewed:

- trusted entropy source and a reviewed cryptographic library;
- boot-image signature verification and a trustworthy key enrollment/rotation/rollback model;
- kernel-enforced capabilities, address spaces, and process transitions;
- a safe update/recovery process and signed artifact provenance;
- sandboxing and resource limits for services and AI tools;
- fuzzing/property tests for boot protocol, ELF parser, memory-map validation and IPC schemas;
- tests for malformed input, overflow, alignment, concurrent calls, failure recovery and privilege boundaries;
- supply-chain license/advisory audits and verified third-party notices;
- explicit release artifacts, hashes, build recipe, supported-target matrix, and rollback instructions.

A green CI run is necessary but not sufficient: it establishes only the checks the workflow actually ran.

## 7. Recommended contributor priorities

1. Finish memory/BootInfo allocator safety and maintain the QEMU/OVMF matrix.
2. Pin the toolchain/actions, run Clippy, triage warnings, preserve test logs and artifacts.
3. Consolidate one paging implementation; keep user-mode isolation disabled until there is a proven mapping lifecycle.
4. Add fault injection for CPU exceptions and later a real timer/interrupt-controller integration.
5. Design device-model and service/IPC interfaces before implementing many drivers.
6. Ship a minimal, useful user-space service/agent client on existing host platforms before giving any agent privileged OS controls.
7. Target named hardware profiles; expand only as boot, drivers, update/recovery, and security tests are added.

## References

- UEFI Forum, [UEFI Specification 2.11](https://uefi.org/specs/UEFI/2.11/) and [overview](https://uefi.org/specs/UEFI/2.11/02_Overview.html).
- Rust Project, [`x86_64-unknown-none` target documentation](https://doc.rust-lang.org/rustc/platform-support/x86_64-unknown-none.html).
- Linux Kernel, [ACPI on Arm systems](https://www.kernel.org/doc/html/latest/arch/arm64/arm-acpi.html) and [device model overview](https://docs.kernel.org/driver-api/driver-model/overview.html).
- OWASP, [MCP Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/MCP_Security_Cheat_Sheet.html).
- systemd project, [service credentials](https://systemd.io/CREDENTIALS/).
