//! Round-Robin Scheduler Implementation
//!
//! This module implements a simple pre-emptive round-robin scheduler
//! that manages thread execution on a single CPU.
//!
//! # Scheduling Algorithm
//!
//! The scheduler uses a classic round-robin algorithm:
//! 1. Maintain a queue of ready threads
//! 2. On each timer tick, move current thread to back of queue
//! 3. Select next thread from front of queue
//! 4. Perform context switch
//!
//! This ensures:
//! - **Fairness**: All threads get equal CPU time
//! - **Pre-emption**: Long-running threads are interrupted
//! - **Responsiveness**: Interactive threads don't starve
//!
//! # Future Enhancements
//!
//! For production, consider:
//! - Multi-level feedback queue (MLFQ) for better interactivity
//! - Priority-based scheduling with priority inheritance
//! - Per-CPU run queues for SMP systems
//! - Real-time scheduling classes (SCHED_FIFO, SCHED_RR)

#![no_std]

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use spin::Mutex;
use crate::task::tcb::{ThreadControlBlock, ThreadState, ThreadId};
use crate::task::context_switch::{switch_context, initialize_thread_stack};
use x86_64::VirtAddr;

/// Maximum number of threads in the system
/// This limits the size of our static arrays
pub const MAX_THREADS: usize = 256;

/// Global scheduler instance
/// Protected by a spinlock for thread-safe access
static SCHEDULER: Mutex<Option<Scheduler>> = Mutex::new(None);

/// The scheduler state
struct Scheduler {
    /// Queue of threads ready to run
    run_queue: VecDeque<*mut ThreadControlBlock>,
    
    /// Pointer to currently running thread
    current: *mut ThreadControlBlock,
    
    /// Idle thread (runs when nothing else is ready)
    idle_thread: *mut ThreadControlBlock,
    
    /// Total number of context switches performed
    switch_count: usize,
    
    /// Number of timer ticks since boot
    tick_count: u64,
}

impl Scheduler {
    /// Create a new scheduler
    fn new() -> Self {
        Self {
            run_queue: VecDeque::new(),
            current: core::ptr::null_mut(),
            idle_thread: core::ptr::null_mut(),
            switch_count: 0,
            tick_count: 0,
        }
    }
    
    /// Get the currently running thread
    #[inline]
    fn current(&self) -> *mut ThreadControlBlock {
        self.current
    }
    
    /// Set the currently running thread
    #[inline]
    fn set_current(&mut self, tcb: *mut ThreadControlBlock) {
        self.current = tcb;
    }
    
    /// Add a thread to the run queue
    fn enqueue(&mut self, tcb: *mut ThreadControlBlock) {
        unsafe {
            if let Some(ref mut thread) = tcb.as_mut() {
                if thread.state == ThreadState::Ready {
                    self.run_queue.push_back(tcb);
                }
            }
        }
    }
    
    /// Remove and return the next thread from the run queue
    fn dequeue(&mut self) -> Option<*mut ThreadControlBlock> {
        self.run_queue.pop_front()
    }
    
    /// Check if there are threads waiting to run
    fn has_ready_threads(&self) -> bool {
        !self.run_queue.is_empty()
    }
    
    /// Increment tick counter
    fn tick(&mut self) {
        self.tick_count += 1;
    }
    
    /// Get total tick count
    fn tick_count(&self) -> u64 {
        self.tick_count
    }
}

/// Initialize the scheduler subsystem
///
/// This must be called before any threads are created.
/// It sets up the idle thread which runs when no other
/// threads are ready.
///
/// # Arguments
///
/// * `idle_tcb` - The idle thread's TCB (must be statically allocated)
pub fn init(idle_tcb: &'static mut ThreadControlBlock) {
    let mut scheduler = SCHEDULER.lock();
    
    // Configure idle thread
    idle_tcb.state = ThreadState::Ready;
    idle_tcb.priority = 0; // Lowest priority
    
    *scheduler = Some(Scheduler::new());
    
    if let Some(ref mut sched) = *scheduler {
        sched.idle_thread = idle_tcb as *mut ThreadControlBlock;
        sched.current = idle_tcb as *mut ThreadControlBlock;
        
        println!("  [Scheduler] Initialized with idle thread");
    }
}

/// Add a new thread to the scheduler
///
/// The thread will be placed in the run queue and scheduled
/// for execution on the next timer tick.
///
/// # Arguments
///
/// * `tcb` - Pointer to the thread's TCB (must have 'static lifetime)
///
/// # Safety
///
/// The TCB pointer must remain valid for the lifetime of the thread.
/// Do not deallocate the TCB until the thread has terminated and
/// been removed from all scheduler data structures.
pub fn spawn(tcb: &'static mut ThreadControlBlock) {
    let mut scheduler = SCHEDULER.lock();
    
    if let Some(ref mut sched) = *scheduler {
        tcb.state = ThreadState::Ready;
        sched.enqueue(tcb as *mut ThreadControlBlock);
        
        println!(
            "  [Scheduler] Spawned thread {} (priority={})",
            tcb.id, tcb.priority
        );
    }
}

/// The main scheduler tick function
///
/// Called by the timer interrupt handler at regular intervals (e.g., 100Hz).
/// This function:
/// 1. Updates timing statistics
/// 2. Moves blocked threads off the run queue
/// 3. Performs round-robin scheduling
/// 4. Triggers a context switch if needed
///
/// # Safety
///
/// This function must be called with interrupts disabled or from
/// an interrupt handler context. Calling it from normal code
/// without proper synchronization will cause race conditions.
pub fn tick() {
    let mut scheduler = SCHEDULER.lock();
    
    if let Some(ref mut sched) = *scheduler {
        sched.tick();
        
        // If we only have the idle thread, no need to switch
        if !sched.has_ready_threads() {
            return;
        }
        
        // Get current thread
        let current_ptr = sched.current();
        
        if current_ptr.is_null() {
            return;
        }
        
        unsafe {
            if let Some(current) = current_ptr.as_mut() {
                // If current thread is still ready, put it at back of queue
                if current.state == ThreadState::Running {
                    current.state = ThreadState::Ready;
                    sched.enqueue(current_ptr);
                }
            }
        }
        
        // Get next thread from front of queue
        if let Some(next_ptr) = sched.dequeue() {
            if !next_ptr.is_null() && next_ptr != current_ptr {
                unsafe {
                    if let Some(next) = next_ptr.as_mut() {
                        // Mark next thread as running
                        next.set_running();
                        
                        // Update current thread pointer
                        sched.set_current(next_ptr);
                        
                        // Increment switch counter
                        sched.switch_count += 1;
                        
                        // Perform the actual context switch
                        // Note: This function doesn't return until this thread
                        // is scheduled again (possibly many ticks later)
                        switch_context(
                            &mut sched.current as *mut *mut ThreadControlBlock,
                            next_ptr,
                        );
                    }
                }
            }
        }
    }
}

/// Get the currently running thread
///
/// # Returns
///
/// Pointer to the current thread's TCB, or null if scheduler not initialized
pub fn current() -> *mut ThreadControlBlock {
    let scheduler = SCHEDULER.lock();
    scheduler
        .as_ref()
        .map(|s| s.current())
        .unwrap_or(core::ptr::null_mut())
}

/// Get the idle thread
///
/// # Returns
///
/// Pointer to the idle thread's TCB, or null if scheduler not initialized
pub fn idle_thread() -> *mut ThreadControlBlock {
    let scheduler = SCHEDULER.lock();
    scheduler
        .as_ref()
        .map(|s| s.idle_thread)
        .unwrap_or(core::ptr::null_mut())
}

/// Block the current thread with a reason
///
/// This removes the current thread from the run queue and
/// marks it as blocked. The scheduler will then select a
/// different thread to run.
///
/// # Arguments
///
/// * `reason` - Why the thread is blocking
pub fn block_current(reason: crate::task::tcb::BlockReason) {
    let mut scheduler = SCHEDULER.lock();
    
    if let Some(ref mut sched) = *scheduler {
        let current_ptr = sched.current();
        
        if !current_ptr.is_null() {
            unsafe {
                if let Some(current) = current_ptr.as_mut() {
                    current.block(reason);
                    
                    // Don't put blocked thread back in run queue
                    // It will be re-added when woken up
                    
                    // Force immediate reschedule
                    drop(scheduler); // Release lock before calling tick
                    tick();
                }
            }
        }
    }
}

/// Wake up a blocked thread
///
/// This moves the thread from blocked state to ready state
/// and adds it to the run queue.
///
/// # Arguments
///
/// * `thread_id` - ID of the thread to wake
///
/// # Returns
///
/// True if the thread was found and woken, false otherwise
pub fn wake_thread(thread_id: ThreadId) -> bool {
    let mut scheduler = SCHEDULER.lock();
    
    if let Some(ref mut sched) = *scheduler {
        // Search for the thread in run queue or current
        // In a real implementation, you'd have a more efficient lookup
        
        // Check current thread
        if !sched.current().is_null() {
            unsafe {
                if let Some(current) = sched.current().as_mut() {
                    if current.id == thread_id && current.state == ThreadState::Blocked {
                        current.wake();
                        sched.enqueue(sched.current());
                        return true;
                    }
                }
            }
        }
        
        // Search run queue
        for tcb_ptr in &mut sched.run_queue {
            unsafe {
                if let Some(tcb) = tcb_ptr.as_mut() {
                    if tcb.id == thread_id && tcb.state == ThreadState::Blocked {
                        tcb.wake();
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

/// Get scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub total_threads: usize,
    pub ready_threads: usize,
    pub blocked_threads: usize,
    pub running_threads: usize,
    pub terminated_threads: usize,
    pub context_switches: usize,
    pub total_ticks: u64,
}

/// Collect scheduler statistics
pub fn get_stats() -> SchedulerStats {
    let scheduler = SCHEDULER.lock();
    
    if let Some(ref sched) = *scheduler {
        // Count threads by state (would need thread list in production)
        SchedulerStats {
            total_threads: sched.run_queue.len() + 1, // +1 for current
            ready_threads: sched.run_queue.len(),
            blocked_threads: 0, // Would need to track separately
            running_threads: 1, // Current thread
            terminated_threads: 0,
            context_switches: sched.switch_count,
            total_ticks: sched.tick_count(),
        }
    } else {
        SchedulerStats {
            total_threads: 0,
            ready_threads: 0,
            blocked_threads: 0,
            running_threads: 0,
            terminated_threads: 0,
            context_switches: 0,
            total_ticks: 0,
        }
    }
}

/// Yield the CPU voluntarily
///
/// This is called by a thread that wants to give up its
/// remaining time slice. Useful for cooperative multitasking
/// or when a thread knows it will be waiting for something.
pub fn yield_now() {
    // Trigger an immediate reschedule
    // In practice, this just calls tick() which does round-robin
    tick();
}

/// Print scheduler debug information
pub fn print_debug_info() {
    let stats = get_stats();
    
    println!("\n=== Scheduler Statistics ===");
    println!("Total threads: {}", stats.total_threads);
    println!("Ready: {}, Blocked: {}, Running: {}", 
             stats.ready_threads, stats.blocked_threads, stats.running_threads);
    println!("Context switches: {}", stats.context_switches);
    println!("Total ticks: {}", stats.total_ticks);
    println!("===========================\n");
}
