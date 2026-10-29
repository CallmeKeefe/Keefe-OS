## Summary

Describe the defect or contribution and the user-visible or engineering outcome.

## Root cause / design

- Root cause:
- Chosen approach:
- Alternatives considered (if relevant):

## Change scope

- [ ] Kernel / unsafe Rust
- [ ] Bootloader / UEFI
- [ ] Memory management / paging
- [ ] CI / tooling
- [ ] Documentation / contributor experience
- [ ] Other:

## Verification

Record exact commands and outcomes. Do not mark checks as passing unless they completed successfully on this change.

- [ ] Host tests
- [ ] Target build(s)
- [ ] QEMU/OVMF boot matrix (if boot, memory-map, or bootloader code changed)
- [ ] RustSec dependency audit (if dependency/workspace files changed)
- [ ] Reviewed relevant CI run for this commit
- [ ] Reviewed final diff for secrets, generated artifacts, and unrelated edits

Commands and results:

```text
# Paste exact commands and concise output here.
```

Not run / blocked checks and why:

## Safety and compatibility

- [ ] No new unsafe code, or every new unsafe block has a documented safety rationale.
- [ ] No destructive disk-writing or physical-install instructions were added without a tested, explicit safety gate.
- [ ] Unsupported hardware / features remain clearly labelled.
- [ ] Docs and status claims were updated to match implementation.

## Reviewer notes

Known limitations, follow-up issue links, screenshots/logs, or reproducible test steps.
