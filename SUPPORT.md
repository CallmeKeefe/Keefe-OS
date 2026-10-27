# Support

KeefeOS is an experimental operating-system prototype. It is not a production OS, a security boundary, or a supported physical-device installation.

## Choose the right channel

- **Build or test failure:** open a [bug report](https://github.com/CallmeKeefe/Keefe-OS/issues/new?template=bug_report.md) with the exact command, commit SHA, host OS, Rust version, QEMU/OVMF versions, and sanitized logs.
- **Feature proposal:** use the [feature request template](https://github.com/CallmeKeefe/Keefe-OS/issues/new?template=feature_request.md).
- **Potential security vulnerability:** do not publish exploit details in a public issue. Follow [SECURITY.md](SECURITY.md) and use GitHub private vulnerability reporting if enabled.
- **Setup and troubleshooting:** read the [Build and Boot Guide](docs/BUILD_AND_BOOT_GUIDE.md), [Compatibility Matrix](docs/COMPATIBILITY.md), and [current OS status](docs/audit/OS_STATUS.md).

## Current support scope

The documented test path is Linux/WSL2 as a build host and x86-64 QEMU + OVMF as the guest environment. Only the documented early-boot milestone has been verified in the cited CI run. Physical installation, recovery, upgrades, broad device support, and production security are not available.

## Reporting a problem

Include the commit SHA, exact steps, expected and actual behavior, failure logs, and whether the issue reproduces on the documented CI path. Redact tokens, credentials, private keys, personal data, and unrelated environment details.

Do not test this prototype on disks containing valuable data.
