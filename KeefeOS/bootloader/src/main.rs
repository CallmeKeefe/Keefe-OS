//! KeefeOS UEFI bootloader.
//!
//! Accepts only the checked low-memory x86-64 ELF format described by the
//! shared boot-protocol crate. It never treats ELF bytes as a PE/COFF image.

#![no_std]
#![no_main]

use core::convert::TryFrom;
use core::mem::size_of;
use core::ptr;
use keefeos_boot_protocol::bootinfo::{BootInfo, MemoryDescriptor, MAX_MEMORY_DESCRIPTORS};
use keefeos_boot_protocol::elf::ElfImage;
use uefi::boot::{self, AllocateType, MemoryType};
use uefi::mem::memory_map::MemoryMap;
use uefi::prelude::*;

static KERNEL_BINARY: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/keefeos-kernel"));

const PAGE_SIZE: u64 = 4096;
const MAX_INFO_ADDRESS: u64 = 0x4000_0000;

#[entry]
fn efi_main() -> Status {
    if let Err(_error) = uefi::helpers::init() {
        return Status::ABORTED;
    }

    log::info!("========================================");
    log::info!("        KeefeOS UEFI loader");
    log::info!("========================================");
    log::info!("Embedded kernel artifact: {} bytes", KERNEL_BINARY.len());

    let image = match ElfImage::parse(KERNEL_BINARY) {
        Ok(image) => image,
        Err(error) => {
            log::error!("Kernel ELF rejected: {:?}", error);
            return Status::LOAD_ERROR;
        }
    };

    let load_start = image.load_start();
    let load_end = image.load_end();
    let Some(span) = load_end.checked_sub(load_start) else {
        log::error!("Invalid ELF load span");
        return Status::LOAD_ERROR;
    };
    if span == 0 || span % PAGE_SIZE != 0 {
        log::error!("ELF load span is not page aligned");
        return Status::LOAD_ERROR;
    }

    let pages = match usize::try_from(span / PAGE_SIZE) {
        Ok(pages) if pages > 0 => pages,
        _ => {
            log::error!("ELF image is too large for this target");
            return Status::LOAD_ERROR;
        }
    };

    let kernel_memory = match boot::allocate_pages(
        AllocateType::Address(load_start),
        MemoryType::LOADER_DATA,
        pages,
    ) {
        Ok(memory) if memory.as_ptr() as u64 == load_start => memory,
        Ok(_) => {
            log::error!("Firmware did not allocate the kernel at its requested physical address");
            return Status::ABORTED;
        }
        Err(error) => {
            log::error!("Could not reserve kernel load range: {:?}", error);
            return Status::ABORTED;
        }
    };

    // SAFETY: allocate_pages returned an exclusive allocation for the validated
    // ELF image span, which is bounded by the shared parser.
    unsafe { ptr::write_bytes(kernel_memory.as_ptr(), 0, span as usize) };

    for index in 0..image.program_header_count() {
        let segment = match image.segment_at(index) {
            Ok(Some(segment)) => segment,
            Ok(None) => continue,
            Err(error) => {
                log::error!("ELF segment validation failed: {:?}", error);
                return Status::LOAD_ERROR;
            }
        };
        let bytes = match image.segment_bytes(segment) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::error!("ELF segment file range invalid: {:?}", error);
                return Status::LOAD_ERROR;
            }
        };
        let Some(destination_offset) = segment.physical_address().checked_sub(load_start) else {
            log::error!("ELF segment begins before reserved kernel range");
            return Status::LOAD_ERROR;
        };
        let destination_offset = match usize::try_from(destination_offset) {
            Ok(offset) => offset,
            Err(_) => return Status::LOAD_ERROR,
        };
        let Some(destination_end) = destination_offset.checked_add(bytes.len()) else {
            return Status::LOAD_ERROR;
        };
        if destination_end > span as usize {
            log::error!("ELF segment exceeds reserved kernel span");
            return Status::LOAD_ERROR;
        }

        // SAFETY: source bytes and destination range were bounds-checked by the
        // ELF parser; the allocated physical destination cannot alias the
        // UEFI application's embedded read-only byte slice.
        unsafe {
            ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                kernel_memory.as_ptr().add(destination_offset),
                bytes.len(),
            );
        }
    }

    let info_pages = (size_of::<BootInfo>() + PAGE_SIZE as usize - 1) / PAGE_SIZE as usize;
    let info_memory = match boot::allocate_pages(
        AllocateType::MaxAddress(MAX_INFO_ADDRESS),
        MemoryType::LOADER_DATA,
        info_pages,
    ) {
        Ok(memory) => memory,
        Err(error) => {
            log::error!("Could not allocate BootInfo: {:?}", error);
            return Status::ABORTED;
        }
    };
    if (info_memory.as_ptr() as usize) % core::mem::align_of::<BootInfo>() != 0 {
        log::error!("Firmware returned a misaligned BootInfo allocation");
        return Status::ABORTED;
    }

    log::info!("Validated ELF entry: {:#x}", image.entry());
    log::info!("Kernel image: {:#x}..{:#x}", load_start, load_end);
    log::info!("Reserved {} kernel pages and {} BootInfo pages", pages, info_pages);
    log::info!("Exiting UEFI boot services and creating normalized BootInfo");

    // SAFETY: all allocations are complete. The returned map is tied to the
    // successful ExitBootServices operation. No boot-service function is used
    // after this point.
    let memory_map = unsafe { boot::exit_boot_services(MemoryType::LOADER_DATA) };

    if memory_map.is_empty() || memory_map.len() > MAX_MEMORY_DESCRIPTORS {
        halt_after_exit();
    }

    let mut boot_info = BootInfo::new();
    let meta = memory_map.meta();
    boot_info.firmware_desc_size = match u32::try_from(meta.desc_size) {
        Ok(size) => size,
        Err(_) => halt_after_exit(),
    };
    boot_info.firmware_desc_version = meta.desc_version;
    boot_info.memory_map_count = memory_map.len() as u32;
    boot_info.kernel_start = load_start;
    boot_info.kernel_end = load_end;
    boot_info.kernel_entry = image.entry();

    for (index, descriptor) in memory_map.entries().enumerate() {
        if index >= MAX_MEMORY_DESCRIPTORS {
            halt_after_exit();
        }
        let phys_start = descriptor.phys_start;
        let page_count = descriptor.page_count;
        let Some(bytes) = page_count.checked_mul(PAGE_SIZE) else {
            halt_after_exit();
        };
        if page_count == 0 || phys_start % PAGE_SIZE != 0
            || phys_start.checked_add(bytes).is_none()
        {
            halt_after_exit();
        }

        boot_info.memory_map[index] = MemoryDescriptor {
            ty: memory_type_code(descriptor.ty),
            _pad: 0,
            phys_start,
            virt_start: descriptor.virt_start,
            page_count,
            att: 0,
        };
    }

    if boot_info.validate(size_of::<BootInfo>()).is_err()
        || boot_info.semantic_check().is_err()
    {
        halt_after_exit();
    }

    // SAFETY: info_memory is a writable LOADER_DATA allocation large enough
    // for BootInfo; alignment was checked above.
    unsafe { ptr::write(info_memory.as_ptr().cast::<BootInfo>(), boot_info) };

    let boot_info_address = info_memory.as_ptr() as u64;
    let entry_address = match usize::try_from(image.entry()) {
        Ok(address) => address,
        Err(_) => halt_after_exit(),
    };

    // The x86-64 contract requires current firmware page tables to retain
    // identity access to the kernel and BootInfo until the kernel builds its own.
    // SAFETY: ELF parsing verified an executable entry within a reserved
    // PT_LOAD segment; BootInfo passed structural and semantic validation.
    let kernel_entry: extern "sysv64" fn(u64) -> ! =
        unsafe { core::mem::transmute(entry_address) };
    kernel_entry(boot_info_address)
}

fn memory_type_code(ty: MemoryType) -> u32 {
    if ty == MemoryType::RESERVED { 0 }
    else if ty == MemoryType::LOADER_CODE { 1 }
    else if ty == MemoryType::LOADER_DATA { 2 }
    else if ty == MemoryType::BOOT_SERVICES_CODE { 3 }
    else if ty == MemoryType::BOOT_SERVICES_DATA { 4 }
    else if ty == MemoryType::RUNTIME_SERVICES_CODE { 5 }
    else if ty == MemoryType::RUNTIME_SERVICES_DATA { 6 }
    else if ty == MemoryType::CONVENTIONAL { 7 }
    else if ty == MemoryType::UNUSABLE { 8 }
    else if ty == MemoryType::ACPI_RECLAIM { 9 }
    else if ty == MemoryType::ACPI_NON_VOLATILE { 10 }
    else if ty == MemoryType::MMIO { 11 }
    else if ty == MemoryType::MMIO_PORT_SPACE { 12 }
    else if ty == MemoryType::PAL_CODE { 13 }
    else if ty == MemoryType::PERSISTENT_MEMORY { 14 }
    else if ty == MemoryType::UNACCEPTED { 15 }
    else { 0 }
}

fn halt_after_exit() -> ! {
    x86_64::instructions::interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn bootloader_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
