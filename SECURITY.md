# Security policy

KeefeOS is an experimental x86-64 operating-system prototype. It is not currently verified to boot and is not a production security boundary. Do not use it to store, process, or protect sensitive information.

## Reporting a vulnerability

Please do not publish an exploitable vulnerability in a public issue. If GitHub private vulnerability reporting is enabled, use the [Security Advisories page](https://github.com/CallmeKeefe/Keefe-OS/security/advisories/new). Otherwise contact the repository owner through their GitHub profile to arrange private disclosure.

Include the affected commit, file/subsystem, platform and build target, reproducible steps, observed impact, and relevant logs. Do not include live credentials, private keys, personal information, or secret values.

## Current security limitations

Cryptographic operations, secure-boot signature verification, capability authorization, runtime vulnerability scanning, and attack detection are not implemented as reliable security controls. APIs that would otherwise imply successful verification fail closed. An empty scan result or a successful compile must not be interpreted as proof of security.

Review the current [security status](docs/audit/SECURITY_STATUS.md) before making claims about any protection.
