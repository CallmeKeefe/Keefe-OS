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
//! The current bootloader deliberately does not yet hand off the ELF kernel.

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]

extern crate alloc;

use alloc::vec::Vec;

use core::panic::PanicInfo;

use keefeos_kernel::{console, interrupts, memory, println};
mod heap;

/// Kernel entry point called by the bootloader
///
/// # Arguments
///
/// * `memory_map` - Pointer to UEFI memory map provided by bootloader
/// * `memory_map_size` - Size of the memory map in bytes
/// * `kernel_start` - Physical start address of the kernel
/// * `kernel_end` - Physical end address of the kernel
///
/// # Safety
///
/// This function is called directly from assembly/UEFI and must not return.
#[unsafe(no_mangle)]
pub extern "C" fn kernel_start(
    memory_map_ptr: usize,
    memory_map_size: usize,
    kernel_start: usize,
    kernel_end: usize,
) -> ! {
    // Initialize console output first (for debugging)
    console::init();
    
    println!("\n========================================");
    println!("   KeefeOS Microkernel v{}", env!("CARGO_PKG_VERSION"));
    println!("========================================\n");
    
    println!("Booting KeefeOS...");
    println!("Kernel loaded at: {:#018x} - {:#018x}", kernel_start, kernel_end);
    let descriptor_size = core::mem::size_of::<UEFIMemoryDescriptor>();
    if descriptor_size == 0 || memory_map_size % descriptor_size != 0 {
        println!("Invalid UEFI memory map size: {}", memory_map_size);
        halt_forever();
    }

    let num_entries = memory_map_size / descriptor_size;
    println!(
        "Memory map at: {:#018x} ({} entries; {} bytes/descriptor)",
        memory_map_ptr, num_entries, descriptor_size
    );
    let mut memory_regions = Vec::with_capacity(num_entries);

    // SAFETY: The bootloader contract must provide a valid, readable UEFI
    // memory-map buffer containing memory_map_size bytes. Each descriptor is
    // read using the exact repr(C) type and descriptor size used by this ABI.
    unsafe {
        let descriptors = memory_map_ptr as *const UEFIMemoryDescriptor;
        for i in 0..num_entries {
            let entry = &*descriptors.add(i);
            let Some(size_bytes) = entry.page_count.checked_mul(4096) else {
                println!("UEFI memory descriptor {} page count overflows", i);
                halt_forever();
            };
            let Ok(size) = usize::try_from(size_bytes) else {
                println!("UEFI memory descriptor {} exceeds addressable memory", i);
                halt_forever();
            };

            memory_regions.push(memory::UEFIMemoryRegion {
                ty: memory::UEFIMemoryType::from(entry.ty),
                phys_start: entry.phys_start as usize,
                size,
            });
        }
    }
    
    println!("Parsed {} memory regions", memory_regions.len());
    
    // Count usable memory
    let mut usable_bytes = 0usize;
    for region in &memory_regions {
        if region.is_usable() {
            usable_bytes = usable_bytes.saturating_add(region.size);
        }
    }
    println!("Total usable RAM: {} MB", usable_bytes / (1024 * 1024));
    
    // Initialize memory management subsystem
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
    println!("Boot milestone withheld: validated loader handoff is not implemented.");
    println!("Entering halt loop.");

    // Halt the CPU indefinitely
    halt_forever();
}

/// UEFI Memory Descriptor structure for the current handoff ABI.
/// The UEFI 2.x layout includes the attributes field; use the type size instead
/// of a magic descriptor stride.
#[repr(C)]
#[derive(Clone, Copy)]
struct UEFIMemoryDescriptor {
    ty: u32,
    _padding: u32,
    phys_start: u64,
    virt_start: u64,
    page_count: u64,
    attributes: u64,
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
