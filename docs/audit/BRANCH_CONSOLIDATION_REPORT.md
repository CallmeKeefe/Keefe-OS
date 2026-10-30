# KeefeOS branch consolidation report

**Snapshot date:** 2026-10-30  
**Comparison baseline:** `main` at `db7e65e974b91fcb895b8f0514a8434f37be122e`  
**Remote branches observed:** 24, including `main`  
**Open pull requests:** #32 (documentation evidence refresh)

This snapshot compares every pre-existing topic branch against the current `main` head. The documentation branch for PR #32 was created during this audit and is listed separately. Commit counts alone do not establish whether code is already integrated: squash merges can leave branch histories divergent even when the changed file contents are identical. Do not merge or delete a divergent branch based on graph counts alone.

At the start of this audit, the 22 pre-existing topic branches classified as follows: 14 behind-only branches with no unique commits, one branch identical to `main`, and seven divergent branches containing unique commits. The audit-created PR #32 branch is three commits ahead and zero behind. This gives 23 topic branches plus `main` (24 total).

## Branch inventory

| Branch | Ahead | Behind | Current disposition |
|---|---:|---:|---|
| `main` | — | — | Canonical branch; keep |
| `aegisos-kernel-design-e9020` | 0 | 118 | Behind-only; no unique commits reported |
| `audit/architecture-security-2026-10` | 107 | 39 | Divergent, very stale; do not merge wholesale; tree/commit review required |
| `audit/architecture-security-review-clean-2026-10` | 38 | 39 | Divergent, very stale; do not merge wholesale; tree/commit review required |
| `audit/installation-readiness-2026-10-27` | 0 | 58 | Behind-only; no unique commits reported |
| `continuous-improvement-process-0743a` | 0 | 106 | Behind-only; no unique commits reported |
| `cpu-exceptions-and-idt-implementation-c01c1` | 1 | 117 | Unique legacy CPU/IDT commit; reviewed below; do not merge as-is |
| `docs/community-publication-prep-2026-10` | 0 | 40 | Behind-only; no unique commits reported |
| `docs/installation-guide-2026-10` | 0 | 56 | Behind-only; no unique commits reported |
| `docs/refresh-ci-evidence-after-dependency-update` | 0 | 0 | Identical to `main`; no work to merge |
| `docs/refresh-ci-security-evidence-2026-10-30` | 3 | 0 | Active documentation branch for PR #32; only README, security status, and this report are changed |
| `docs/update-latest-ci-evidence` | 1 | 2 | Stale README-only evidence change; superseded by PR #32 |
| `fix/installability-security-gates` | 0 | 59 | Behind-only; no unique commits reported |
| `fix/uefi-041-deps-current-main` | 3 | 1 | Divergent history, but all three changed file contents are identical to current `main`; no cherry-pick needed |
| `improve-all-branch-codes-b6a21` | 0 | 100 | Behind-only; no unique commits reported |
| `improvement-suggestions-40545` | 0 | 104 | Behind-only; no unique commits reported |
| `memory-management-implementation-eecab` | 0 | 116 | Behind-only; no unique commits reported |
| `microkernel-os-development-da1f5` | 1 | 115 | Unique legacy multitasking commit; reviewed below; do not merge as-is |
| `next-gen-secure-os-enhancement-1998a` | 0 | 110 | Behind-only; no unique commits reported |
| `os-development-and-deployment-guide-0841e` | 0 | 108 | Behind-only; no unique commits reported |
| `os-enhancement-with-ai-integration-826cf` | 0 | 112 | Behind-only; no unique commits reported |
| `security-vulnerability-assessment-82d9d` | 0 | 102 | Behind-only; no unique commits reported |
| `uefi-kernel-boot-fixes-6dac8` | 0 | 95 | Behind-only; no unique commits reported |
| `ultimate-os-development-blueprint-9a520` | 1 | 111 | Unique legacy blueprint/legal/docs commit; reviewed below; do not merge as-is |

## Unique legacy commits reviewed

### CPU / IDT — `29f96e2c0326996092b24e8867b17a7cbc28e929`

The branch assigns the double-fault stack to TSS array index 1 while the IDT requests IST index 1. In x86-64, IDT IST index 1 selects TSS array slot 0, so these indices are inconsistent. The branch also renames an initializer call without a matching exported interface in its accompanying tree. Current `main` uses TSS slot 0. Do not merge this commit.

### Multitasking — `00d4c548bec829972c171fcc3d57d31fb4605ed2`

The branch adds PIT timer, TCB, and context-switch prototypes. It introduces timer interrupt and low-level context-switch code while current `main` intentionally keeps device IRQ routing disabled. The new code requires a written register/stack ABI, interrupt-frame invariants, lock/interrupt ordering, scheduler-state ownership, and isolated QEMU tests plus dedicated unsafe-code review. Do not merge without that work.

### Security / blueprint documentation — `145cc55d0cd71eaa9d999140e5607af2d72ecfa1`

The unique commit adds documentation that states or implies completed secure boot, memory isolation, broad hardware support, or a completed security audit without matching implementation evidence. It also carries stale platform build commands and dependency/license metadata. Do not import these claims as facts.

### UEFI dependency branch — `fix/uefi-041-deps-current-main`

The branch graph is divergent (3 ahead / 1 behind), but direct content comparison found `KeefeOS/Cargo.toml`, `KeefeOS/Cargo.lock`, and `KeefeOS/bootloader/src/main.rs` byte-for-byte identical to current `main`. Its resulting tree changes are already present; no cherry-pick or merge is needed.

### Audit branches

The two audit branches contain broad changes across kernel code, build configuration, and documentation, with 39 commits behind `main`. Their commit counts and file lists do not prove that every individual change is already represented in the current tree. Do not merge them wholesale. Extract only a specifically identified, still-missing fix after comparing its exact patch against current code and running the full test matrix.

### Old CI documentation branch

`docs/update-latest-ci-evidence` contains one README-only commit based on an older main revision. PR #32 now refreshes the README and security status using the verified 2026-10-29 CI evidence. Do not merge the stale branch.

## Current PR #32

[PR #32 — refresh verified CI and CodeQL evidence](https://github.com/CallmeKeefe/Keefe-OS/pull/32) updates the README, security status, and this branch report. The latest PR head is documentation-only and three commits ahead of `main`. Its CI and CodeQL workflows were queued at the time of this snapshot; do not merge until required checks complete successfully.

## Recommended disposition

1. Keep `main` as the canonical source branch. Do not force-push or rewrite its history.
2. Merge PR #32 only after its build/QEMU and CodeQL checks complete successfully.
3. Do not merge the CPU/IDT, multitasking, or blueprint commits wholesale. Extract individual changes only after current-code review and tests.
4. No merge is needed for the UEFI dependency branch because its changed file contents already match `main`.
5. Review the two large audit branches at patch level before deciding whether any unique fix is worth extracting.
6. The 14 behind-only branches are cleanup candidates only after confirming that no contributor, workflow, release, or review depends on them.

## Remote deletion limitation

The connected GitHub operations available in this session support branch search/comparison and repository file/commit operations, but do not expose a branch-delete action. No remote branch was deleted in this audit. Branch cleanup remains incomplete and must be performed through GitHub's Branches page or an authorized Git client after review. No branch was force-rewritten to imitate deletion.

This is an API-backed branch graph and selected unique-commit review, not a claim that every line in every historical branch has been exhaustively audited.


## Addendum — PR #32 merged after the inventory snapshot

On 2026-10-30, PR [#32](https://github.com/CallmeKeefe/Keefe-OS/pull/32) was merged by squash as commit `e888340a6e6b6ec734fb62ee0eb55d881fcc4088`. Its README CI evidence, security-status update, and this branch inventory are now present on `main`.

The PR head passed the Build/QEMU workflow and both CodeQL analysis jobs before merge. The original table above remains a point-in-time comparison against the pre-merge `main` baseline `db7e65e974b91fcb895b8f0514a8434f37be122e`; it is not a refreshed comparison against the new `main` commit. The merged documentation branch is now a cleanup candidate, but no remote branch was deleted.
