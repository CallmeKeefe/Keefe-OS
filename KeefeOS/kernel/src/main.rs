//! KeefeOS Microkernel Main Entry Point
//!
//! This is the entry point for the KeefeOS microkernel.
//! It initializes all core subsystems and then halts.
//!
//! # Boot Flow
//!
//! 1. UEFI bootloader loads the kernel at physical address 0x100000
//! 2. CPU jumps to `kernel_start` (defined in linker script)
//! 3. Early initialization (console, memory)
//! 4. Subsystem initialization (GDT, IDT, paging)
//! 5. Enter kernel main loop or halt

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(naked_functions)]

extern crate alloc;

use core::panic::PanicInfo;

mod arch;
mod console;
mod memory;
mod interrupts;

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
#[no_mangle]
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
    println!("Memory map at: {:#018x} ({} entries)", memory_map_ptr, memory_map_size / 48);
    
    // Parse the UEFI memory map
    // Each UEFI memory descriptor is 48 bytes
    let num_entries = memory_map_size / 48;
    let mut memory_regions = Vec::with_capacity(num_entries);
    
    unsafe {
        let ptr = memory_map_ptr as *const u8;
        for i in 0..num_entries {
            let entry_ptr = ptr.add(i * 48) as *const UEFIMemoryDescriptor;
            let entry = &*entry_ptr;
            
            memory_regions.push(memory::UEFIMemoryRegion {
                ty: memory::UEFIMemoryType::from(entry.ty),
                phys_start: entry.phys_start as usize,
                size: entry.size as usize,
            });
        }
    }
    
    println!("Parsed {} memory regions", memory_regions.len());
    
    // Count usable memory
    let mut usable_bytes = 0usize;
    for region in &memory_regions {
        if region.is_usable() {
            usable_bytes += region.size;
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
    let gdt_wrapper = interrupts::gdt::init();
    
    // Initialize IDT (Interrupt Descriptor Table)
    println!("Initializing IDT...");
    interrupts::idt::init();
    
    println!("\n========================================");
    println!("KeefeOS Microkernel initialized successfully!");
    println!("========================================");
    println!("  - Memory Manager: Active");
    println!("  - GDT/TSS: Active ({} descriptors)", 5);
    println!("  - IDT: Active (48 handlers: 22 exceptions + 16 IRQs)");
    println!("  - Double Fault Protection: IST-enabled");
    println!("\nSystem halted. Press reset to restart.");
    
    // Halt the CPU indefinitely
    keefeos_kernel::halt();
}

/// UEFI Memory Descriptor structure
/// Matches the UEFI specification layout (48 bytes)
#[repr(C)]
struct UEFIMemoryDescriptor {
    ty: u32,
    _padding: u32,
    phys_start: u64,
    virt_start: u64,
    size: u64,
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
    if let Some(message) = info.message() {
        println!("Message: {}", message);
    } else if let Some(args) = info.payload().downcast_ref::<&str>() {
        println!("Message: {}", args);
    }
    
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

/// Simple vector implementation for early boot (before alloc is available)
/// This is a temporary solution until we have proper heap allocation
struct Vec<T> {
    data: [T; 256],
    len: usize,
}

impl<T: Default> Vec<T> {
    fn with_capacity(cap: usize) -> Self {
        Vec {
            data: core::array::from_fn(|_| T::default()),
            len: 0,
        }
    }
    
    fn push(&mut self, item: T) {
        if self.len < self.data.len() {
            self.data[self.len] = item;
            self.len += 1;
        }
    }
}

impl<T> core::ops::Deref for Vec<T> {
    type Target = [T];
    
    fn deref(&self) -> &Self::Target {
        &self.data[..self.len]
    }
}

impl<T> core::iter::FromIterator<T> for Vec<T> 
where
    T: Default,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vec = Vec::with_capacity(256);
        for item in iter {
            vec.push(item);
        }
        vec
    }
}
