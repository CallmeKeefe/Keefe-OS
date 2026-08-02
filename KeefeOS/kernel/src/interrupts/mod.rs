//! KeefeOS Interrupt Subsystem
//!
//! This module provides the complete interrupt handling infrastructure:
//! - Interrupt Descriptor Table (IDT) setup
//! - CPU exception handlers (divide by zero, page fault, etc.)
//! - Double fault handler with dedicated Interrupt Stack Table (IST)
//! - Hardware interrupt groundwork (IRQ offset mapping)
//! - Timer interrupt configuration (PIT)
//!
//! # Capability-Based Security Design
//!
//! In KeefeOS's microkernel architecture:
//! - Ring 0 (kernel) exceptions trigger controlled panics to protect system integrity
//! - Ring 3 (user-space) exceptions will eventually terminate only the faulting process
//!   and send an IPC message to a user-space crash handler
//!
//! The IDT entries themselves are capability-protected: only the kernel can modify them.
//! User-space processes cannot install their own handlers - they must go through
//! approved IPC mechanisms.
//!
//! # Exception Vector Layout
//!
//! ```text
//! Vectors 0-31:   CPU Exceptions (non-maskable)
//! Vectors 32-47:  Hardware IRQs (8259 PIC legacy, remapped)
//!                 - IRQ 0 (Timer/PIT) -> Vector 32
//!                 - IRQ 1 (Keyboard) -> Vector 33
//!                 - ...
//! Vectors 48+:    Software interrupts, syscalls, APIC interrupts
//! ```
//!
//! # Double Fault Prevention
//!
//! A double fault occurs when an exception happens while handling another exception.
//! Common causes:
//! - Stack overflow during exception handling
//! - Missing exception handler
//! - Corrupted IDT/GDT
//!
//! We prevent triple-fault reboot by:
//! 1. Using a dedicated Interrupt Stack Table (IST) entry for double faults
//! 2. This stack is guaranteed to be valid even if the main stack is corrupted
//! 3. The TSS (Task State Segment) configures automatic stack switching

#![no_std]

pub mod idt;
pub mod gdt;
pub mod exceptions;
pub mod timer;

// Re-export key types for convenience
pub use idt::init_interrupts;
pub use gdt::{GlobalDescriptorTableWrapper, TaskStateSegmentWrapper};
pub use exceptions::ExceptionFrame;
pub use timer::{init_pit, DEFAULT_TIMER_FREQUENCY, PIT_BASE_FREQUENCY};

/// Hardware interrupt offset
/// 
/// We remap the 8259 PIC to start at vector 32 to avoid collision with
/// CPU exceptions (vectors 0-31). This is critical because:
/// - Vector 0-31 are reserved for CPU exceptions
/// - IRQ 0 (timer) would normally be at vector 8 (conflicts with Double Fault!)
/// - IRQ 1 (keyboard) would normally be at vector 9
///
/// By remapping to 32 (0x20):
/// - IRQ 0 (timer) -> Vector 32
/// - IRQ 1 (keyboard) -> Vector 33
/// - ...
/// - IRQ 15 -> Vector 47
pub const HARDWARE_INTERRUPT_OFFSET: u8 = 0x20;

/// Number of hardware IRQs on legacy 8259 PIC
pub const NUM_IRQS: usize = 16;

/// Calculate the vector number for a given IRQ
#[inline]
pub const fn irq_to_vector(irq: u8) -> u8 {
    HARDWARE_INTERRUPT_OFFSET + irq
}
