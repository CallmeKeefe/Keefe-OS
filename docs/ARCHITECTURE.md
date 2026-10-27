# KeefeOS architecture: current implementation and target direction

**Status:** Architecture guide for an experimental project. The target design below is not a claim that the features are already implemented. The verified target remains x86-64 QEMU + OVMF early boot.

## 1. The guiding rule

Keep the trusted kernel small. The kernel owns low-level mechanisms such as memory safety boundaries, CPU setup, scheduling primitives, IPC enforcement, and device ownership. Networking, user applications, AI inference, MCP clients, API integrations, and agent workflows belong in isolated user-space services once that infrastructure exists.

Do not put a general-purpose LLM client, API key, MCP server, shell executor, browser, package installer, or autonomous agent loop inside Ring 0. A malicious prompt, compromised provider, malformed tool response, or buggy agent must not become a kernel compromise.

## 2. What exists today

The Git repository has a single Cargo workspace under `KeefeOS/`:

- `boot-protocol/`: shared BootInfo ABI and bounds-checked ELF64 parser.
- `bootloader/`: x86-64 UEFI loader; reserves the fixed kernel image range, loads validated ELF segments, creates BootInfo, and exits boot services.
- `kernel/`: experimental x86-64 kernel code for boot handoff, console output, memory management, CPU exception setup, and prototype subsystem interfaces.
- `xtask/`: host-side image packaging and QEMU helper.
- `Cargo.toml` and `Cargo.lock`: workspace dependency graph and lockfile.
- `.cargo/config.toml` and `rust-toolchain.toml`: target/build setup.

Repository-level documentation is authoritative for community policy and the current limitations. The nested `KeefeOS/README.md` is a workspace-specific build overview. Older design notes are proposals, not feature guarantees.

### Current boot path

1. UEFI firmware starts the loader.
2. The loader parses the embedded x86-64 ELF using the shared parser.
3. It reserves the project's fixed 32–64 MiB kernel load window and validates segment bounds.
4. It builds a BootInfo handoff and exits UEFI boot services.
5. The kernel validates the handoff and reads the memory map.
6. QEMU/OVMF tests require the kernel boot marker. The firmware can render a splash before ExitBootServices; the persistent direct kernel framebuffer remains disabled when the memory-map range cannot be proven safe.

The last verified baseline before this review was [run 40600247257](https://github.com/CallmeKeefe/Keefe-OS/actions/runs/40600247257) on commit `3c474110c427233c0d6b73373848fd7bb34d4142`. This audit branch adds code and CI changes that need their own run. The baseline result is still a QEMU-based early-boot check, not proof of a complete OS, physical hardware support, or a security boundary.

## 3. Intended layering

The following is a target architecture to implement incrementally.

### Layer A — Firmware and architecture abstraction

- Separate architecture backends such as x86-64 and AArch64, each with its own boot protocol, exception/trap setup, interrupt controller, page-table implementation, linker rules, and architecture-specific tests.
- Keep firmware-specific code and CPU-specific operations behind small, documented interfaces.
- Avoid claiming that one kernel image can boot every device. Every supported device needs an explicit architecture, firmware, and driver combination.

### Layer B — Kernel mechanisms

- Validated boot handoff and memory-map ownership.
- Physical frame allocation, virtual memory mapping, and an explicitly documented address-space model.
- Exception handling and device interrupt routing.
- Scheduler/context switching only after stack, ABI, interrupt, lock, and ownership invariants have tests.
- IPC and capability enforcement before untrusted user processes or agent tools can be isolated.
- Fail closed when safety conditions cannot be established.

These mechanisms are not all complete. See [OS status](audit/OS_STATUS.md), [security status](audit/SECURITY_STATUS.md), and [compatibility](COMPATIBILITY.md).

### Layer C — User-space services (future)

After process isolation and system-call/IPC boundaries exist, build services outside the kernel:

- device manager and driver services;
- storage, filesystem, networking, input, display, and audio;
- init/service supervision, configuration, package updates, logging, and recovery;
- interactive shell and desktop/mobile UI;
- credential/key storage and access policy;
- AI agent runtime and provider/tool adapters.

Each service should have a narrow interface, restart behavior, bounded resource use, and tests for malformed requests and failure. A user-space boundary is not real until the kernel actually enforces it.

### Layer D — AI agents and tool integrations (future)

The AI runtime should call a local policy broker through a documented IPC API. The broker checks user identity, granted capabilities, permitted resources, action risk, and confirmation requirements before any tool executes. See [AI agent security model](AI_AGENT_SECURITY_MODEL.md).

MCP and ordinary APIs are transports/interfaces for tool access, not authorization systems. The operating system must enforce permissions locally even if the model or remote service requests otherwise.

## 4. Portability strategy

“Works on all devices” must be approached as a tested support matrix, not a broad compatibility badge.

1. Stabilize the current x86-64 QEMU/OVMF path.
2. Add virtual devices and kernel/service interfaces with automated tests.
3. Validate a small, explicit list of x86-64 UEFI PCs only after storage, input, graphics, networking, firmware variation, and recovery are addressed.
4. Develop AArch64 as a separate architecture backend and boot path; begin with an emulated machine before selecting specific physical boards.
5. Treat phones/tablets as separate hardware targets. Boot chains, vendor drivers, secure boot, modem/GPU/touch, power management, app distribution, and recovery differ by device. Android/iOS apps are not equivalent to replacing those operating systems.
6. Add other architectures such as RISC-V only when they have code, CI, and reproducible boot evidence.

Support statements must name the exact board/VM, architecture, firmware, device set, and tested commit. Never infer physical-device support from a successful QEMU boot.

## 5. Workspace and code-organization rules

- Keep the shared boot ABI in `boot-protocol`; do not duplicate the loader/kernel structure definitions.
- Keep `no_std` kernel crates free of network access, provider credentials, model downloads, and user-facing agent loops.
- Put architecture-specific code behind architecture modules/crates; do not scatter target checks throughout unrelated business logic.
- Prefer safe wrappers around unsafe operations, with precise preconditions and tests at arithmetic, alignment, lifetime, and bounds edges.
- Avoid a second active page-table implementation. The legacy `arch/paging.rs` feature is explicitly disabled; do not enable it until it is reconciled with the active memory subsystem and tests.
- Keep generated images, build output, credentials, and local state out of Git.
- Every public feature claim should point to source, tests, and CI evidence.

## 6. First architecture milestones

See [the staged roadmap](ROADMAP.md) and the linked engineering issues. The immediate work is correctness and honest scope—not putting more ambitious subsystem names into the kernel.