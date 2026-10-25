//! KeefeOS UEFI loader entry point.
//!
//! Safety policy: the current kernel artifact is a generic ELF image, not a
//! UEFI PE/COFF application. Until a tested ELF loader and BootInfo handoff
//! exist, this program refuses to jump to the ELF image base as an entry point.

#![no_std]
#![no_main]

use uefi::prelude::*;

static KERNEL_BINARY: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/keefeos-kernel"));

#[entry]
fn efi_main() -> Status {
    if let Err(_error) = uefi::helpers::init() {
        return Status::ABORTED;
    }

    log::info!("========================================");
    log::info!("        KeefeOS UEFI loader");
    log::info!("========================================");
    log::info!("Embedded kernel artifact: {} bytes", KERNEL_BINARY.len());

    if !KERNEL_BINARY.starts_with(b"\x7fELF") {
        log::error!("Kernel artifact is not an ELF image; refusing handoff");
        return Status::LOAD_ERROR;
    }

    log::error!("Boot blocked safely: validated ELF64 loading and BootInfo handoff are not yet integrated.");
    log::error!("The loader will not execute the ELF image base as though it were the entry point.");
    log::error!("KeefeOS is not yet bootable. See docs/audit/INITIAL_CODEBASE_AUDIT.md.");
    Status::UNSUPPORTED
}

#[panic_handler]
fn bootloader_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
