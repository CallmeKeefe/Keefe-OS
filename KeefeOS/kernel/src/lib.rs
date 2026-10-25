//! KeefeOS Microkernel Library
//!
//! This is the experimental core library of the KeefeOS kernel.
//! It provides safe Rust abstractions for:
//! - Physical memory management (frame allocation)
//! - Virtual memory management (page tables)
//! - Interrupt handling (GDT, IDT, exceptions)
//! - Capability-based security primitives
//! - AI/ML-powered intelligent subsystems
//! - Advanced security with blockchain audit logs
//! - LLM-powered AI agent for natural language interaction
//!
//! # Safety Guarantees
//!
//! Hardware access is still experimental and some low-level APIs remain unsafe.
//! `unsafe` blocks are only used where absolutely necessary:
//! - CPU-specific instructions (CR3, INVLPG, etc.)
//! - Memory-mapped I/O access
//! - Inline assembly for low-level operations
//!
//! Unsafe code requires ongoing review; the project is not a production-ready OS.

#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate alloc;

pub mod arch;
pub mod memory;
pub mod console;
pub mod interrupts;
pub mod ai;
pub mod security;
pub mod llm_agent;

/// Kernel version information
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KERNEL_NAME: &str = "KeefeOS Microkernel";

/// Initialize the kernel subsystems
/// This function should be called after basic hardware initialization
pub fn init(
    memory_map: &[memory::UEFIMemoryRegion],
    kernel_start: usize,
    kernel_end: usize,
) {
    log::info!("Initializing {} v{}", KERNEL_NAME, KERNEL_VERSION);

    // The boot caller supplies the validated memory map and image bounds.
    memory::init(memory_map, kernel_start, kernel_end);
    
    // Initialize AI/ML subsystem
    ai::init();
    
    // Initialize security subsystem
    security::init().unwrap_or_else(|e| {
        log::error!("Failed to initialize security subsystem: {}", e);
    });
    
    // Initialize LLM-powered AI agent
    llm_agent::init();
    
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
