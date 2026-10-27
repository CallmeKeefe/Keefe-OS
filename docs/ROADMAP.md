# KeefeOS roadmap

**Status:** Proposed milestones, not promises or completed features. Acceptance evidence must come from source, tests, and CI.

## Principle

Build a small, testable operating-system foundation first, then grow the user-space services and agent ecosystem. Supporting many devices requires architecture-specific boot code, hardware abstraction, drivers, platform integration, and a device-by-device test matrix. It will not be achieved by one universal image or a collection of feature names.

## Stage 0 — Reproducible experimental x86-64 boot

**Current baseline:** x86-64 UEFI early boot under QEMU + OVMF, with 256 MiB, 512 MiB, and 1 GiB RAM profiles in the documented CI run.

Acceptance:
- repeatable clean-checkout build/test commands;
- parser, BootInfo, and kernel unit tests pass;
- QEMU marker and splash are checked for every supported profile;
- test logs/artifacts are retained;
- unsupported hardware and limitations are stated accurately.

## Stage 1 — Kernel correctness and memory safety

Priority work is tracked in [#19](https://github.com/CallmeKeefe/Keefe-OS/issues/19), [#21](https://github.com/CallmeKeefe/Keefe-OS/issues/21), and [#23](https://github.com/CallmeKeefe/Keefe-OS/issues/23).

Acceptance:
- audited BootInfo, memory-map, frame allocator, page-table, and address-conversion invariants;
- tests cover overflow, alignment, reserved/overlapping ranges, invalid descriptors, and allocator edge cases;
- all unsafe blocks have precise safety rationale and relevant regression tests;
- failed safety checks leave the system in a safe failure state;
- compiler warnings are triaged rather than hidden.

## Stage 2 — Minimal OS mechanisms

Implement and verify exception behavior, device IRQ routing, context switching, address-space lifecycle, and IPC in small increments. Avoid integrating old branch prototypes until the exact diff has passed the current test suite.

Acceptance:
- scheduler and context-switch ABI documented and tested;
- process/user transition and isolation verified by negative tests;
- IPC authenticates the sender and checks authorization;
- memory mappings cannot cross address spaces without an explicit kernel-enforced grant.

## Stage 3 — First virtual device platform and user space

Start with a narrow, reproducible QEMU x86-64 target. Prefer a well-defined virtual device set before attempting broad physical hardware.

Candidate services: init/service supervision, console/shell, storage and filesystem, networking, input/display, package/update process, logs, recovery, and configuration.

Acceptance:
- each driver/service has error handling and bounded resource use;
- tests simulate missing, malformed, and failed devices;
- service crashes do not corrupt the kernel or unrelated services;
- disk image creation is separate from successful boot, installation, and recovery claims.

## Stage 4 — Secure service and permission framework

Track security foundations in [#20](https://github.com/CallmeKeefe/Keefe-OS/issues/20), and dependency/license/provenance checks in [#25](https://github.com/CallmeKeefe/Keefe-OS/issues/25). Use maintained cryptographic components and independent review; do not invent custom cryptography.

Acceptance:
- trusted entropy and key management are implemented for the chosen target;
- boot trust, signing, update verification, and rollback policy have a threat model and test vectors;
- capabilities are checked by the kernel and are not caller-supplied labels;
- update/recovery behavior is tested;
- the project continues to state that security is incomplete until independently audited.

## Stage 5 — Personal agent runtime and MCP/API integrations

See [the AI agent security model](AI_AGENT_SECURITY_MODEL.md) and implementation work tracked in [#24](https://github.com/CallmeKeefe/Keefe-OS/issues/24). The agent belongs in user space, not the kernel. Do not begin unrestricted command execution before process isolation, IPC authorization, sandboxing, and credential storage exist.

First useful MVP:
- local UI for a conversation and a visible action plan;
- read-only typed status tools;
- optional provider adapters for hosted APIs or supported local inference;
- MCP client for explicitly approved servers and tools;
- tool allowlists, per-tool permission grants, rate/size/time limits, audit events, cancel, and dry-run;
- human approval for file writes, network sends, software installation, and system configuration;
- no arbitrary root shell and no silent background destructive actions.

Acceptance:
- agent is optional; boot and core functions work without an LLM, API key, or Internet;
- user data sent to providers is disclosed and minimized;
- malicious prompt/tool output cannot bypass local policy;
- each reported action is verified against its postcondition.

## Stage 6 — Add architectures and specific hardware

Architecture support must be added separately. Recommended order:
1. keep x86-64 QEMU + OVMF stable;
2. expand the x86-64 virtual device matrix;
3. prototype AArch64 on an emulated machine, with its own boot protocol, exception model, linker, and CI;
4. select named physical x86-64/AArch64 boards only after their drivers and recovery path are ready;
5. pursue RISC-V or mobile targets as separate projects/milestones with concrete hardware evidence.

“Phone/tablet support” needs a specific target device, unlock/boot-chain plan, signed image/recovery story, GPU/display/touch/network/power drivers, and user-data protections. An Android/iOS application is a companion app, not a native OS replacement.

## Stage 7 — Cross-device ecosystem

Only after secure identity, networking, device discovery, permissions, service lifecycle, and recovery exist should KeefeOS attempt cross-device tools, shared resources, remote agent tasks, or a device fabric.

Acceptance:
- mutual authentication, explicit pairing/revocation, encrypted transport, least-privilege remote resource grants;
- clear trust and data-flow UI;
- one compromised device cannot automatically control all other devices;
- interoperability and failure behavior tested across named platforms.

## Release gates

Repository-wide documentation consolidation, license/provenance review, and stale-branch dispositions are tracked in [#26](https://github.com/CallmeKeefe/Keefe-OS/issues/26). A milestone is complete only when:
- code is merged through review;
- required CI passes on the exact merged commit;
- tests and artifacts can be reproduced;
- docs distinguish implemented, experimental, planned, and unsupported capabilities;
- security and compatibility claims match the evidence.