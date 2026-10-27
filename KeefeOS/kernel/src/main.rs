//! KeefeOS Microkernel Main Entry Point
//!
//! This is the entry point for the KeefeOS microkernel.
//! It initializes all core subsystems and then halts.
//!
//! # Boot Flow
//!
//! Intended flow once a valid loader handoff exists:
//! 1. Validate boot information and the memory map.
//! 2. Initialize serial diagnostics and reserved/usable memory ranges.
//! 3. Initialize GDT/TSS and the CPU exception table.
//! 4. Emit a serial-visible milestone and enter the idle halt loop.
//!
//! The current loader passes the validated BootInfo handoff and transfers control
//! to this entry point. The tested early-boot path remains experimental.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::vec::Vec;

use core::panic::PanicInfo;

use keefeos_kernel::{console, interrupts, memory, println};
mod heap;

/// Kernel entry point using the shared KeefeOS BootInfo ABI.
///
/// # Safety
/// The bootloader must provide a readable, aligned BootInfo allocation that
/// remains reserved and identity-addressable during early initialization.
#[unsafe(no_mangle)]
pub extern "sysv64" fn kernel_start(boot_info_ptr: u64) -> ! {
    console::init();
    println!("\n========================================");
    println!("   KeefeOS Microkernel v{}", env!("CARGO_PKG_VERSION"));
    println!("========================================\n");
    println!("Validating KeefeOS BootInfo handoff...");

    if !keefeos_kernel::bootinfo::is_valid_boot_info_pointer(boot_info_ptr) {
        println!("Invalid, out-of-contract, misaligned, or truncated BootInfo pointer");
        halt_forever();
    }

    // SAFETY: the loader contract provides an identity-addressable BootInfo
    // allocation within the validated low-memory range above.
    let boot_info = unsafe {
        &*(boot_info_ptr as *const keefeos_kernel::bootinfo::BootInfo)
    };
    if let Err(error) = boot_info.validate(core::mem::size_of::<keefeos_kernel::bootinfo::BootInfo>()) {
        println!("BootInfo validation failed: {:?}", error);
        halt_forever();
    }
    if let Err(error) = boot_info.semantic_check() {
        println!("BootInfo semantic validation failed: {:?}", error);
        halt_forever();
    }

    // The UEFI bootloader renders the startup splash through firmware GOP BLT.
    // Do not write to the physical framebuffer directly here: a "reserved" UEFI
    // memory descriptor does not prove that the current page tables map the range
    // or that its MMIO cache attributes are correct.
    if boot_info.flags & keefeos_kernel::bootinfo::BOOT_INFO_FLAG_GOP_FOUND == 0 {
        println!("No UEFI GOP protocol found; serial diagnostics remain active");
    } else if boot_info.flags & keefeos_kernel::bootinfo::BOOT_INFO_FLAG_GOP_CONVENTIONAL_OVERLAP != 0 {
        println!("GOP range overlaps conventional RAM; direct kernel writes refused");
    } else if boot_info.flags & keefeos_kernel::bootinfo::BOOT_INFO_FLAG_GOP_UNMAPPED != 0 {
        println!("GOP range has a gap in the final memory map; direct kernel writes refused");
    } else if boot_info.flags & keefeos_kernel::bootinfo::BOOT_INFO_FLAG_GOP_RESERVED != 0 {
        println!("GOP range is reserved in the firmware map, but direct kernel framebuffer writes remain disabled until page-table and MMIO attributes are verified");
    } else {
        println!("GOP framebuffer safety could not be proven; serial-only output");
    }

    let kernel_start = boot_info.kernel_start as usize;
    let kernel_end = boot_info.kernel_end as usize;
    println!("Kernel loaded at: {:#018x} - {:#018x}", kernel_start, kernel_end);
    println!(
        "Firmware map: {} descriptors; descriptor size {} bytes",
        boot_info.memory_map_count, boot_info.firmware_desc_size
    );

    let count = boot_info.memory_map_count as usize;
    let mut memory_regions = Vec::with_capacity(count);
    for descriptor in boot_info.memory_map[..count].iter() {
        let size_bytes = match descriptor.page_count.checked_mul(keefeos_kernel::bootinfo::PAGE_SIZE) {
            Some(size) => size,
            None => {
                println!("Memory descriptor page count overflow");
                halt_forever();
            }
        };
        let size = match usize::try_from(size_bytes) {
            Ok(size) => size,
            Err(_) => {
                println!("Memory descriptor size exceeds addressable memory");
                halt_forever();
            }
        };
        let phys_start = match usize::try_from(descriptor.phys_start) {
            Ok(address) => address,
            Err(_) => {
                println!("Memory descriptor start exceeds addressable memory");
                halt_forever();
            }
        };
        memory_regions.push(memory::UEFIMemoryRegion {
            ty: memory::UEFIMemoryType::from(descriptor.ty),
            phys_start,
            size,
        });
    }

    let usable_bytes = memory_regions
        .iter()
        .filter(|region| region.is_usable())
        .fold(0usize, |total, region| total.saturating_add(region.size));
    println!(
        "Validated {} memory regions; conventional RAM: {} MiB",
        memory_regions.len(), usable_bytes / (1024 * 1024)
    );

    memory::init(&memory_regions, kernel_start, kernel_end);

    // Print memory statistics
    let stats = memory::get_memory_stats();
    println!("Memory Stats:");
    println!("  Total frames: {}", stats.total_frames);
    println!("  Free frames: {}", stats.free_frames);
    println!("  Total memory: {} GB", stats.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
    
    // Initialize GDT (Global Descriptor Table) and TSS
    println!("Initializing GDT and TSS...");
    let _gdt_wrapper = interrupts::gdt::init();
    
    // Initialize IDT (Interrupt Descriptor Table)
    println!("Initializing IDT...");
    interrupts::idt::init();
    
    // Keep early boot deterministic; experimental AI/LLM modules are not
    // required for boot and are not security controls.
    println!("[NOT IMPLEMENTED] Secure-boot signature verification");
    println!("[NOT IMPLEMENTED] Authenticated encryption and trusted random generation");
    println!("[FAIL CLOSED] Unverified capability grants are denied");
    println!("[NOT IMPLEMENTED] Runtime vulnerability scanning and attack detection");

    // Run initial security audit
    println!("Running initial security audit...");
    let audit_report = keefeos_kernel::security::security_audit();
    println!("{}", audit_report);
    
    println!("\n========================================");
    println!("KeefeOS kernel initialization reached idle path");
    println!("Memory manager: initialized (prototype; paging safety not verified)");
    println!("GDT/TSS: initialized; user-mode support is not implemented");
    println!("CPU exception table: initialized; device IRQ routing is disabled");
    println!("Security status: INCOMPLETE; no production security boundary");
    // Do not emit the CI boot-success marker until the UEFI loader actually
    // validates the ELF, constructs BootInfo, exits boot services, and transfers
    // control to this entry point.
    println!("KEEFEOS-KERNEL-BOOT-OK");
    println!("Entering halt loop.");

    // Halt the CPU indefinitely
    halt_forever();
}

/// Panic handler - called when a panic occurs
///
/// This handler prints diagnostic information and halts the system.
/// In a production system, you might want to:
/// - Log the panic to persistent storage
/// - Attempt recovery if possible
/// - Reboot the system
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts to prevent nested panics
    x86_64::instructions::interrupts::disable();
    
    println!("\n");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!         KERNEL PANIC              !!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");
    
    // Print the panic message
    println!("Message: {}", info.message());
    
    // Print location if available
    if let Some(location) = info.location() {
        println!(
            "Location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }
    
    println!("\nSystem halted. Please reset your computer.");
    
    // Halt the CPU
    loop {
        x86_64::instructions::hlt();
    }
}

/// Allocation failures are fatal until a reclaiming heap allocator exists.
#[alloc_error_handler]
fn alloc_error(layout: core::alloc::Layout) -> ! {
    println!(
        "KERNEL ALLOCATION FAILURE: size={}, align={}",
        layout.size(),
        layout.align()
    );
    halt_forever()
}

/// Disable interrupts and halt after a fatal early-boot error.
fn halt_forever() -> ! {
    x86_64::instructions::interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}
