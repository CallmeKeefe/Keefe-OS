//! KeefeOS experimental kernel library.
//!
//! This crate contains early kernel mechanisms plus experimental interfaces.
//! It is not yet a complete microkernel: user-mode processes, enforced
//! capability isolation, a general driver framework, and production paging
//! lifecycle are not verified.
//!
//! The AI modules are deterministic heuristics and a local command demo.
//! They are not a connected LLM, agent runtime, process scheduler, or security
//! monitor. The security module fails closed but does not implement a trusted
//! boot chain or a complete cryptographic/key-management system.
//!
//! Hardware access remains experimental. Unsafe operations require ongoing
//! review and tests tied to their actual safety invariants.

#![no_std]
#![feature(asm_const)]
#![feature(naked_functions)]
#![feature(panic_info_message)]
#![feature(abi_x86_interrupt)]
#![warn(missing_docs)]
#![warn(missing_debug_implementations)]

extern crate alloc;

pub mod bootinfo;
pub mod arch;
pub mod memory;
pub mod console;
pub mod interrupts;
pub mod ai;
pub mod security;
pub mod llm_agent;

/// Kernel version information
pub const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const KERNEL_NAME: &str = "KeefeOS experimental kernel";

/// Initialize the current experimental kernel subsystems.
///
/// This initializes memory helpers, deterministic AI heuristics, security
/// status reporting, and a local command demo. It does not initialize a
/// general user-space runtime, connect an external LLM, or enable a production
/// security boundary.
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
