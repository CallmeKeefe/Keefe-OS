//! Thread Control Block (TCB) - Core Thread State Structure
//!
//! The TCB is the kernel's primary data structure for tracking thread state.
//! It contains all information needed to save/restore a thread during context switches
//! and manage its execution lifecycle.
//!
//! # Design Philosophy
//!
//! In KeefeOS, threads are the fundamental unit of:
//! - **Scheduling**: The scheduler operates on threads, not processes
//! - **Resource Accounting**: CPU time, memory limits tracked per-thread
//! - **Capability Ownership**: Each thread has its own capability space
//!
//! A "process" in KeefeOS is simply a collection of threads that share:
//! - Virtual address space (page tables)
//! - File descriptors
//! - Capability namespace
//!
//! # Memory Layout
//!
//! ```text
//! +------------------+  <- High addresses
//! |   Stack Guard    |     (unmapped page for overflow detection)
//! +------------------+
//! |                  |
//! |   Thread Stack   |     Grows downward
//! |        ↓         |
//! |                  |
//! +------------------+  <- RSP points here during execution
//! |   TCB Struct     |     Kernel-managed metadata
//! +------------------+  <- Low addresses
//! ```
//!
//! # Capability-Based Security
//!
//! Each TCB contains a `CapabilitySpace` that defines what resources
//! the thread can access:
//! - Memory pages it can map
//! - IPC endpoints it can send/receive from
//! - I/O ports it can access (for driver threads)
//! - Child threads/processes it controls
//!
//! This design ensures that even if a thread is compromised,
//! its damage is limited to its capability set.

#![no_std]

use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use x86_64::{VirtAddr, PhysAddr};
use x86_64::structures::paging::PageTable;

/// Unique identifier for each thread in the system
///
/// Thread IDs are monotonically increasing and never reused
/// (until wraparound after 2^64 threads, which won't happen).
pub type ThreadId = u64;

/// Global counter for generating unique thread IDs
static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique thread ID
#[inline]
pub fn generate_thread_id() -> ThreadId {
    NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed)
}

/// Thread execution states
///
/// These states form a simple state machine:
/// ```text
/// Ready ←→ Running → Blocked → Ready
///    ↓
/// Terminated (absorbing state)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ThreadState {
    /// Thread is ready to run but not currently executing
    /// Placed in the scheduler's run queue
    Ready = 0,
    
    /// Thread is currently executing on a CPU
    /// Only one thread per CPU can be in this state
    Running = 1,
    
    /// Thread is waiting for an event:
    /// - IPC message arrival
    /// - Timer expiration
    /// - I/O completion
    /// - Lock acquisition
    ///
    /// Blocked threads are removed from the run queue
    /// until the awaited event occurs.
    Blocked = 2,
    
    /// Thread has exited and is being cleaned up
    /// Resources will be reclaimed once all references are dropped
    Terminated = 3,
}

impl ThreadState {
    /// Check if this thread can be scheduled
    #[inline]
    pub const fn is_schedulable(&self) -> bool {
        matches!(self, Self::Ready)
    }
    
    /// Check if this thread is currently executing
    #[inline]
    pub const fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
    
    /// Convert to human-readable string
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Running => "Running",
            Self::Blocked => "Blocked",
            Self::Terminated => "Terminated",
        }
    }
}

/// Reason why a thread is blocked
///
/// This helps with debugging and proper wake-up handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// Waiting for an IPC message on an endpoint
    WaitingForIPC { endpoint_id: u64 },
    
    /// Waiting for a timer to expire
    WaitingForTimer { wake_time: u64 },
    
    /// Waiting for I/O operation to complete
    WaitingForIO { device_id: u32 },
    
    /// Waiting to acquire a lock/semaphore
    WaitingForLock { lock_id: usize },
    
    /// Blocked by explicit yield (sleep)
    Sleeping { wake_time: u64 },
    
    /// Other/unspecified reason
    Other,
}

/// Capability Space - Per-Thread Authority Table
///
/// This structure holds all capabilities owned by the thread.
/// Capabilities are unforgeable tokens that grant specific permissions:
/// - MemoryPage: Right to access/map a physical page
/// - Endpoint: Right to send/receive IPC messages
/// - IOPort: Right to access specific hardware I/O ports
/// - Process: Right to control/signal a process
///
/// # Implementation Notes
///
/// For now, we use a fixed-size array for simplicity.
/// In production, you would use:
/// - A hash table for O(1) capability lookup
/// - Hierarchical capability spaces for delegation
/// - Revocation lists for efficient capability revocation
#[derive(Debug, Clone)]
pub struct CapabilitySpace {
    /// Number of valid capabilities in the table
    count: usize,
    
    /// Fixed-size capability table (placeholder)
    /// TODO: Replace with proper capability implementation in Step 5
    capabilities: [u128; 64], // Placeholder for capability handles
}

impl CapabilitySpace {
    /// Create a new empty capability space
    pub const fn new() -> Self {
        Self {
            count: 0,
            capabilities: [0; 64],
        }
    }
    
    /// Get the number of capabilities held
    #[inline]
    pub const fn count(&self) -> usize {
        self.count
    }
    
    /// Check if capability space is full
    #[inline]
    pub const fn is_full(&self) -> bool {
        self.count >= self.capabilities.len()
    }
    
    /// Insert a capability (placeholder - to be implemented in Step 5)
    #[inline]
    pub fn insert(&mut self, cap: u128) -> Result<(), ()> {
        if self.is_full() {
            return Err(());
        }
        self.capabilities[self.count] = cap;
        self.count += 1;
        Ok(())
    }
}

impl Default for CapabilitySpace {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread Control Block
///
/// This is the complete state needed to manage a thread.
/// During context switch, we save CPU registers into this struct
/// and restore them from another thread's TCB.
///
/// # Fields Explanation
///
/// ## Execution State
/// - `id`: Unique thread identifier
/// - `state`: Current scheduling state (Ready/Running/Blocked)
/// - `block_reason`: Why the thread is blocked (if applicable)
///
/// ## Register Save Area
/// - `stack_pointer`: Current RSP (updated during execution)
/// - `instruction_pointer`: Saved RIP (where to resume)
/// - `registers`: General purpose registers (r12-r15, rbx, rbp, etc.)
///
/// ## Resource Management
/// - `stack_top`: Top of thread's stack (for deallocation)
/// - `stack_size`: Size of allocated stack
/// - `priority`: Scheduling priority (future use)
///
/// ## Capability-Based Security
/// - `capabilities`: Authority table defining accessible resources
///
/// ## Parent/Child Relationships
/// - `parent_tid`: Parent thread (for process hierarchy)
/// - `process_id`: Owning process (threads in same process share address space)
#[derive(Debug)]
pub struct ThreadControlBlock {
    /// Unique thread identifier
    pub id: ThreadId,
    
    /// Current execution state
    pub state: ThreadState,
    
    /// Reason for being blocked (if state == Blocked)
    pub block_reason: Option<BlockReason>,
    
    /// Thread name for debugging (optional)
    pub name: Option<&'static str>,
    
    // ================================================================
    // REGISTER SAVE AREA (used during context switch)
    // ================================================================
    
    /// Stack pointer (RSP) - current position in thread's stack
    /// This is the ONLY field that changes during normal execution
    pub stack_pointer: VirtAddr,
    
    /// Instruction pointer (RIP) - where to resume execution
    /// Only valid when thread is not running
    pub instruction_pointer: VirtAddr,
    
    /// Callee-saved registers (preserved across function calls)
    /// These must be saved/restored during context switch:
    /// - RBX: Base register (often points to data structures)
    /// - RBP: Base pointer (stack frame reference)
    /// - R12-R15: Additional callee-saved registers (System V AMD64 ABI)
    pub saved_registers: [u64; 7], // rbx, rbp, r12, r13, r14, r15, rflags
    
    // ================================================================
    // STACK MANAGEMENT
    // ================================================================
    
    /// Virtual address of the top of the thread's stack
    /// Used for stack deallocation when thread terminates
    pub stack_top: VirtAddr,
    
    /// Size of the allocated stack in bytes
    /// Standard sizes: 8KB (minimal), 16KB (default), 32KB (large)
    pub stack_size: usize,
    
    /// Physical address of the stack (for page table management)
    pub stack_phys: PhysAddr,
    
    // ================================================================
    // CAPABILITY-BASED SECURITY
    // ================================================================
    
    /// This thread's capability space
    /// Defines what resources it can access
    pub capabilities: CapabilitySpace,
    
    // ================================================================
    // PARENT/CHILD RELATIONSHIPS
    // ================================================================
    
    /// Parent thread ID (0 for init/orphan threads)
    pub parent_tid: ThreadId,
    
    /// Process ID this thread belongs to
    /// Threads in the same process share:
    /// - Page tables (virtual address space)
    /// - File descriptor table
    /// - Capability namespace root
    pub process_id: u64,
    
    // ================================================================
    // SCHEDULING METADATA
    // ================================================================
    
    /// Scheduling priority (higher = more CPU time)
    /// Range: 0 (lowest) to 255 (highest)
    /// Default: 128 (normal priority)
    pub priority: u8,
    
    /// Accumulated CPU time in milliseconds (for accounting)
    pub cpu_time_ms: AtomicU64,
    
    /// Number of times this thread has been scheduled
    pub schedule_count: AtomicUsize,
}

// Safety: TCB can be sent between CPUs for load balancing
// (though we don't implement that yet)
unsafe impl Send for ThreadControlBlock {}
unsafe impl Sync for ThreadControlBlock {}

impl ThreadControlBlock {
    /// Create a new TCB for a thread
    ///
    /// # Arguments
    ///
    /// * `stack_top` - Virtual address of the top of the thread's stack
    /// * `entry_point` - Virtual address where thread execution begins
    /// * `stack_phys` - Physical address of the stack base
    /// * `stack_size` - Size of the stack in bytes
    /// * `process_id` - ID of the owning process
    ///
    /// # Returns
    ///
    /// A new TCB with initial state set to `Ready`
    pub fn new(
        stack_top: VirtAddr,
        entry_point: VirtAddr,
        stack_phys: PhysAddr,
        stack_size: usize,
        process_id: u64,
    ) -> Self {
        let id = generate_thread_id();
        
        Self {
            id,
            state: ThreadState::Ready,
            block_reason: None,
            name: None,
            
            // Initial register state
            // Stack pointer starts at the top (stack grows downward)
            stack_pointer: stack_top,
            instruction_pointer: entry_point,
            saved_registers: [0; 7], // All zeros initially
            
            // Stack info
            stack_top,
            stack_size,
            stack_phys,
            
            // Empty capability space
            capabilities: CapabilitySpace::new(),
            
            // Parent/child
            parent_tid: 0,
            process_id,
            
            // Scheduling
            priority: 128, // Default normal priority
            cpu_time_ms: AtomicU64::new(0),
            schedule_count: AtomicUsize::new(0),
        }
    }
    
    /// Create a new TCB with a custom name (for debugging)
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
    
    /// Set the thread's parent
    pub fn with_parent(mut self, parent_tid: ThreadId) -> Self {
        self.parent_tid = parent_tid;
        self
    }
    
    /// Get the thread's current state
    #[inline]
    pub const fn state(&self) -> ThreadState {
        self.state
    }
    
    /// Check if this thread is the idle thread (ID 0)
    #[inline]
    pub const fn is_idle(&self) -> bool {
        self.id == 0
    }
    
    /// Mark the thread as blocked with a reason
    #[inline]
    pub fn block(&mut self, reason: BlockReason) {
        self.state = ThreadState::Blocked;
        self.block_reason = Some(reason);
    }
    
    /// Wake up a blocked thread, making it ready to run
    #[inline]
    pub fn wake(&mut self) {
        if self.state == ThreadState::Blocked {
            self.state = ThreadState::Ready;
            self.block_reason = None;
        }
    }
    
    /// Mark the thread as running (called by scheduler)
    #[inline]
    pub fn set_running(&mut self) {
        self.state = ThreadState::Running;
    }
    
    /// Mark the thread as terminated
    #[inline]
    pub fn terminate(&mut self) {
        self.state = ThreadState::Terminated;
    }
    
    /// Increment CPU time counter (called by timer interrupt)
    #[inline]
    pub fn add_cpu_time(&self, ms: u64) {
        self.cpu_time_ms.fetch_add(ms, Ordering::Relaxed);
    }
    
    /// Increment schedule count (called by scheduler)
    #[inline]
    pub fn increment_schedule_count(&self) {
        self.schedule_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for ThreadControlBlock {
    fn default() -> Self {
        Self::new(
            VirtAddr::new(0),
            VirtAddr::new(0),
            PhysAddr::new(0),
            0,
            0,
        )
    }
}

/// Thread stack size constants
pub mod stack_sizes {
    /// Minimal stack for simple threads (8KB)
    pub const MINIMAL: usize = 8 * 1024;
    
    /// Default stack size for most threads (16KB)
    pub const DEFAULT: usize = 16 * 1024;
    
    /// Large stack for complex operations (32KB)
    pub const LARGE: usize = 32 * 1024;
    
    /// Extra-large stack for kernel workers (64KB)
    pub const EXTRA_LARGE: usize = 64 * 1024;
    
    /// Double fault IST stack (from GDT module)
    pub const DOUBLE_FAULT: usize = 32 * 1024;
}
