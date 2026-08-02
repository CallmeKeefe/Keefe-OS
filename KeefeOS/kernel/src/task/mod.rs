//! Task Management Subsystem
//!
//! This module provides all functionality for thread management:
//! - Thread Control Block (TCB) definition
//! - Context switching (assembly implementation)
//! - Round-robin scheduler
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Scheduler                                │
//! │  +-------------------------------------------------------+  │
//! │  |  Run Queue (VecDeque)                                 |  │
//! │  |  [Thread A] → [Thread B] → [Thread C] → ...          |  │
//! │  +-------------------------------------------------------+  │
//! │                          │                                  │
//! │                          ▼ tick()                           │
//! │  +-------------------------------------------------------+  │
//! │  |  Context Switch                                        |  │
//! │  |  1. Save current registers to TCB                      |  │
//! │  |  2. Load next thread's registers from TCB              |  │
//! │  |  3. Switch stack pointer                               |  │
//! │  +-------------------------------------------------------+  │
//! └─────────────────────────────────────────────────────────────┘
//!                          │
//!         ┌────────────────┼────────────────┐
//!         ▼                ▼                ▼
//!   ┌──────────┐    ┌──────────┐    ┌──────────┐
//!   │ Thread A │    │ Thread B │    │ Thread C │
//!   │   TCB    │    │   TCB    │    │   TCB    │
//!   │  Stack   │    │  Stack   │    │  Stack   │
//!   └──────────┘    └──────────┘    └──────────┘
//! ```
//!
//! # Capability-Based Security Integration
//!
//! Each thread has a `CapabilitySpace` that defines its authority:
//! - What memory it can access
//! - What IPC endpoints it can use
//! - What I/O ports it can access
//!
//! The scheduler respects these boundaries when making scheduling decisions.

#![no_std]

pub mod tcb;
pub mod context_switch;
pub mod scheduler;

pub use tcb::{
    ThreadControlBlock,
    ThreadState,
    BlockReason,
    CapabilitySpace,
    ThreadId,
    generate_thread_id,
};

pub use scheduler::{
    init as scheduler_init,
    spawn,
    tick,
    current,
    block_current,
    wake_thread,
    yield_now,
    get_stats,
    print_debug_info,
    SchedulerStats,
};

/// Initialize the entire task subsystem
///
/// # Arguments
///
/// * `idle_tcb` - The idle thread's TCB (must be statically allocated)
pub fn init(idle_tcb: &'static mut ThreadControlBlock) {
    crate::console::println!("Initializing task subsystem...");
    scheduler_init(idle_tcb);
    crate::console::println!("  [Task] Subsystem initialized");
}
