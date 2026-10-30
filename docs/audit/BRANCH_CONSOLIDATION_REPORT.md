# KeefeOS branch consolidation report

**Snapshot date:** 2026-10-30  
**Comparison baseline:** `main` at `b7b9600d87e310a29015fdfa94ae9d39d9507953`  
**Existing branch inventory:** 24 unique topic branches plus `main` were surfaced and compared.  
**Existing open pull requests:** none at the time of this scan.

This refresh supersedes the older point-in-time branch counts below the PR #32 addendum. The comparison baseline is current `main` at the SHA above. Git history counts alone do not prove whether code is safe or already integrated: squash merges can leave histories divergent even when tree contents match. Do not merge or delete a divergent branch based only on ahead/behind counts.

## Current branch inventory

| Branch | Ahead | Behind | Current disposition |
|---|---:|---:|---|
| `main` | — | — | Canonical branch; keep |
| `aegisos-kernel-design-e9020` | 0 | 120 | Behind-only; no unique commits reported |
| `audit/architecture-security-2026-10` | 107 | 41 | Highly divergent; broad kernel/build/docs changes; do not merge wholesale |
| `audit/architecture-security-review-clean-2026-10` | 38 | 41 | Highly divergent; broad kernel/build/docs changes; do not merge wholesale |
| `audit/installation-readiness-2026-10-27` | 0 | 60 | Behind-only; no unique commits reported |
| `continuous-improvement-process-0743a` | 0 | 108 | Behind-only; no unique commits reported |
| `cpu-exceptions-and-idt-implementation-c01c1` | 1 | 119 | One unique legacy CPU/IDT commit; do not merge as-is |
| `docs/community-publication-prep-2026-10` | 0 | 42 | Behind-only; no unique commits reported |
| `docs/installation-guide-2026-10` | 0 | 58 | Behind-only; no unique commits reported |
| `docs/record-pr32-merge-2026-10-30` | 1 | 1 | Divergent; one branch-report change; inspect before cleanup |
| `docs/refresh-ci-evidence-after-dependency-update` | 0 | 2 | Behind-only; no unique commits reported |
| `docs/refresh-ci-security-evidence-2026-10-30` | 4 | 2 | Divergent, documentation-only paths: README, branch report, security status; review exact patch |
| `docs/update-latest-ci-evidence` | 1 | 4 | Divergent, README-only stale evidence change; likely superseded, verify content before cleanup |
| `fix/installability-security-gates` | 0 | 61 | Behind-only; no unique commits reported |
| `fix/uefi-041-deps-current-main` | 3 | 3 | Divergent; Cargo manifests/lockfile and UEFI loader changed; recheck tree before deciding |
| `improve-all-branch-codes-b6a21` | 0 | 102 | Behind-only; no unique commits reported |
| `improvement-suggestions-40545` | 0 | 106 | Behind-only; no unique commits reported |
| `memory-management-implementation-eecab` | 0 | 118 | Behind-only; no unique commits reported |
| `microkernel-os-development-da1f5` | 1 | 117 | One unique legacy multitasking commit; do not merge as-is |
| `next-gen-secure-os-enhancement-1998a` | 0 | 112 | Behind-only; no unique commits reported |
| `os-development-and-deployment-guide-0841a` | — | — | Branch appeared in search but compare returned 404; unresolved, do not assume deleted |
| `os-enhancement-with-ai-integration-826cf` | 0 | 114 | Behind-only; no unique commits reported |
| `security-vulnerability-assessment-82d9d` | 0 | 104 | Behind-only; no unique commits reported |
| `uefi-kernel-boot-fixes-6dac8` | 0 | 97 | Behind-only; no unique commits reported |
| `ultimate-os-development-blueprint-9a520` | 1 | 113 | One unique legacy blueprint/legal/docs commit; do not merge as-is |

The 14 behind-only branches have no unique commits according to GitHub's compare endpoint and are cleanup candidates after checking that no workflow, contributor, release, or review depends on them. Nine branches are divergent and must be reviewed at patch/tree level before merge or deletion. One listed branch could not be compared because GitHub returned 404. This is not a claim that every line in every historical branch has been exhaustively audited.

## Unique legacy work and safety notes

### CPU / IDT

The unique CPU/IDT work was previously reviewed. It assigned the double-fault stack to TSS array index 1 while the IDT requested IST index 1; x86-64 IST index 1 selects TSS array slot 0. The branch also renamed an initializer without a matching exported interface in its accompanying tree. Current `main` uses TSS slot 0. Do not merge this legacy commit without a fresh code review and QEMU regression tests.

### Multitasking

The unique microkernel commit adds PIT timer, TCB, and context-switch prototypes. It introduces timer interrupt and low-level context-switch code while current `main` intentionally keeps device IRQ routing disabled. Before any extraction, require a written register/stack ABI, interrupt-frame invariants, lock/interrupt ordering, scheduler-state ownership, isolated QEMU tests, and dedicated unsafe-code review.

### Security / blueprint documentation

The unique blueprint commit was previously found to state or imply completed secure boot, memory isolation, broad hardware support, or a completed security audit without matching implementation evidence. It also carried stale platform build commands and dependency/license metadata. Do not import those claims as facts.

### Large audit branches

The two large audit branches are 107/41 and 38/41 ahead/behind. Their file lists cover kernel code, build configuration, and documentation. Do not merge either branch wholesale. Extract only a specific fix after comparing its exact patch with current code and running the full test matrix.

### UEFI dependency branch

The UEFI branch is now 3 ahead / 3 behind. An older snapshot found its changed file contents matched `main`, but that finding predates the current comparison baseline. Recheck the three changed paths before treating it as redundant.

### Documentation branches

The CI evidence branches contain overlapping README/security/report changes. PR #32 has already been merged; however, the remaining branches are divergent or behind-only as shown above. Compare file contents before deciding which are redundant. Do not merge stale CI claims without matching them to current successful workflow runs.

## Recommended disposition

1. Keep `main` canonical. Do not force-push or rewrite its history.
2. Do not merge the CPU/IDT, multitasking, large audit, or blueprint branches wholesale.
3. Review the nine divergent branches selectively; preserve unique work until its value and safety are established.
4. The 14 behind-only branches are candidates for deletion only after confirming there are no external dependencies.
5. Investigate the listed installation-guide branch's 404 before taking any cleanup action.
6. Keep documentation evidence tied to current CI runs; do not describe QEMU-only success as physical-device support or security certification.

## Remote branch deletion limitation

The connected GitHub operations available in this session support branch search, compare, and repository file/commit operations, but do not expose a branch-delete action. No remote branch was deleted. Cleanup of behind-only branches must be performed through GitHub's Branches page or an authorized Git client after review. No branch was force-rewritten to imitate deletion.

## Scope and limitations

This report is an API-backed branch graph comparison plus selected prior unique-commit review. It is not a complete line-by-line audit of every historical branch. Branch search pagination repeated the final `ultimate-os-development-blueprint-9a520` entry on the continuation page; the inventory above deduplicates it. The audit refresh itself is proposed separately and is not part of the baseline comparison.
