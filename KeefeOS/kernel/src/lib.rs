//! KeefeOS Microkernel Library
//!
//! This is the core library of the KeefeOS microkernel.
//! It provides safe Rust abstractions for:
//! - Physical memory management (frame allocation)
//! - Virtual memory management (page tables)
//! - Interrupt handling (GDT, IDT, exceptions)
//! - Capability-based security primitives
//!
//! # Safety Guarantees
//!
//! All hardware access is encapsulated in safe interfaces.
//! `unsafe` blocks are only used where absolutely necessary:
//! - CPU-specific instructions (CR3, INVLPG, etc.)
//! - Memory-mapped I/O access
//! - Inline assembly for low-level operations
//!
//! All unsafe code is isolated and thoroughly documented.

#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate alloc;

pub mod arch;
pub mod memory;
pub mod console;
pub mod interrupts;

/// Kernel version information
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KERNEL_NAME: &str = "KeefeOS Microkernel";

/// Initialize the kernel subsystems
/// This function should be called after basic hardware initialization
pub fn init() {
    log::info!("Initializing {} v{}", KERNEL_NAME, KERNEL_VERSION);
    
    // Initialize memory subsystem
    memory::init();
    
    log::info!("KeefeOS initialization complete");
}

/// Halt the CPU indefinitely
/// Used in panic handlers and when the kernel has nothing to do
#[inline(always)]
pub fn halt() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
