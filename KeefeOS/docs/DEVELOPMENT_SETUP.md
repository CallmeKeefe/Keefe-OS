# KeefeOS development setup

This guide covers compiling the experimental kernel and creating a test disk image. It does not describe a supported end-user installation: bootability is not verified yet.

## Supported development path

The current CI and QEMU helper are Linux-oriented. On Windows, use WSL2 with an Ubuntu distribution for the documented build/image flow. macOS and native Windows builds have not been validated as part of this setup.

### 1. Install prerequisites

Install Rust nightly and the components/targets referenced by the repository's rust-toolchain.toml:

~~~sh
rustup toolchain install nightly --component rust-src --component llvm-tools --component rustfmt --component clippy --target x86_64-unknown-none --target x86_64-unknown-uefi
~~~

For Ubuntu/Debian, install QEMU and OVMF:

~~~sh
sudo apt update
sudo apt install qemu-system-x86 ovmf
~~~

Check that commands are available:

~~~sh
rustc +nightly --version
cargo +nightly --version
qemu-system-x86_64 --version
~~~

The QEMU check is needed only to attempt emulation; the compile and image steps can be run without launching a VM.

### 2. Build in the correct order

From the repository root:

~~~sh
cd KeefeOS
cargo +nightly fmt --all -- --check
cargo +nightly check --locked --package xtask --target x86_64-unknown-linux-gnu
cargo +nightly build --locked --package keefeos-kernel --target x86_64-unknown-none
cargo +nightly build --locked --package keefeos-bootloader --target x86_64-unknown-uefi
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- image
~~~

Build the kernel first because the bootloader build script embeds the kernel artifact. Use the same profile for both packages. For release builds, add --release to each of the kernel, bootloader, and image commands.

The generated image is normally at KeefeOS/target/keefeos.img. Do not commit generated images or build directories.

### 3. Attempt QEMU

After installing QEMU and OVMF, run:

~~~sh
cargo +nightly run --locked --package xtask --target x86_64-unknown-linux-gnu -- run
~~~

The helper checks common Linux OVMF paths. If the firmware files are installed elsewhere, configure the helper rather than assuming a missing file is a kernel failure.

**Important:** a QEMU process starting is not proof of a successful boot. Record serial output and the exact last confirmed milestone. Stop and report errors rather than describing a hung VM as successful.

## Known limitations

- The bootloader currently treats an embedded ELF kernel as if it were a UEFI-loadable image and derives the kernel entry address incorrectly. This must be fixed before the current boot path can be considered functional.
- The kernel's memory-map parser hard-codes a descriptor size and mismatches parts of the UEFI descriptor layout. Correct the ABI and test it before trusting memory statistics or frame allocation.
- The higher-half address helpers and current linker load address do not yet demonstrate a coherent virtual-memory layout.
- The workflow builds and packages an image; it does not currently prove that the image boots in QEMU.
- Architecture proposals and module names are not proof that process isolation, a full scheduler, a driver stack, secure boot, encryption services, or AI agents are production-ready.

Track evidence and remediation in docs/audit/INITIAL_CODEBASE_AUDIT.md.
