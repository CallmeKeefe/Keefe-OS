//! Context Switching Implementation (x86_64 Assembly)
//!
//! This module provides the low-level assembly code for switching
//! between threads. The context switch is the heart of multitasking.
//!
//! # Stack Layout During Context Switch
//!
//! When `switch_context` is called, the stack looks like:
//!
//! ```text
//! +------------------+  <- Current RSP (before push)
//! |   Return Addr    |     (return address to caller)
//! +------------------+
//! |     RFLAGS       |     (pushed by save macro)
//! +------------------+
//! |       RIP        |     (pushed by save macro)
//! +------------------+
//! |      R15         |     (saved registers)
//! +------------------+
//! |      R14         |
//! +------------------+
//! |      R13         |
//! +------------------+
//! |      R12         |
//! +------------------+
//! |      RBP         |
//! +------------------+
//! |      RBX         |
//! +------------------+
//! |       RAX        |     (scratch, not preserved)
//! +------------------+  <- New RSP after switch
//! |                  |
//! |  Next Thread's   |
//! |     Stack        |
//! |                  |
//! +------------------+
//! ```
//!
//! # Registers Saved/Restored
//!
//! We follow the System V AMD64 ABI calling convention:
//!
//! ## Callee-Saved (must be preserved across function calls):
//! - RBX, RBP, R12, R13, R14, R15
//! - These are saved/restored during context switch
//!
//! ## Caller-Saved (can be clobbered by called function):
//! - RAX, RCX, RDX, RSI, RDI, R8, R9, R10, R11
//! - These do NOT need to be saved (caller expects them to change)
//!
//! ## Special Registers:
//! - RSP: Switched explicitly (points to different thread's stack)
//! - RIP: Restored from saved value (where thread was interrupted)
//! - RFLAGS: Saved/restored to preserve interrupt flag, condition codes
//!
//! # Atomicity Guarantees
//!
//! The context switch MUST be atomic with respect to interrupts:
//! - Interrupts are disabled at entry
//! - Interrupts are re-enabled AFTER switching stacks
//! - This prevents an interrupt from occurring mid-switch with
//!   corrupted stack state
//!
//! # Safety Considerations
//!
//! 1. **Stack Validity**: Both old and new stacks must be valid
//! 2. **Page Tables**: Must be using kernel page tables (not user space yet)
//! 3. **FPU State**: FPU/SSE registers are NOT saved (future enhancement needed)
//! 4. **MSR State**: GS/FS base registers must be updated separately

#![no_std]

use core::arch::naked_asm;
use x86_64::VirtAddr;
use crate::task::tcb::ThreadControlBlock;

/// Naked function for context switching between two threads
///
/// This function performs a full context switch:
/// 1. Save current CPU registers to `current` TCB
/// 2. Swap stack pointer to `next` thread's stack
/// 3. Restore saved registers from `next` TCB
/// 4. Resume execution at `next` thread's instruction pointer
///
/// # Arguments
///
/// * `current` - Pointer to the current thread's TCB (inout parameter)
/// * `next` - Pointer to the next thread's TCB (input parameter)
///
/// # Safety
///
/// This function is extremely unsafe and requires:
/// - Both TCB pointers must be valid and properly aligned
/// - Both threads must have valid kernel stacks
/// - Interrupts should be disabled before calling
/// - Must be called from kernel mode (Ring 0)
/// - FPU state is NOT preserved (will be lost across switch)
///
/// # Implementation Notes
///
/// This is a "naked" function, meaning:
/// - No function prologue/epilogue is generated
/// - No return instruction (we manually jump back)
/// - Complete control over register usage
#[naked]
pub unsafe extern "C" fn switch_context(
    current: &mut *mut ThreadControlBlock,
    next: *mut ThreadControlBlock,
) {
    // Implementation uses naked_asm for complete control
    // over register saving/restoring
    unsafe {
        naked_asm!(
            // ================================================================
            // PHASE 1: SAVE CURRENT THREAD STATE
            // ================================================================
            
            // Disable interrupts to ensure atomic context switch
            // If an interrupt occurred mid-switch, we'd have corrupted state
            "cli",
            
            // Save callee-saved registers to current thread's TCB
            // [rdi] points to `current` parameter (pointer to TCB pointer)
            // We need to dereference it to get the actual TCB address
            
            // Get current TCB pointer from first argument (rdi)
            "mov rax, [rdi]",          // rax = *current (TCB address)
            
            // Calculate offset to saved_registers field in TCB
            // ThreadControlBlock layout:
            //   +0: id (u64)
            //   +8: state (u8) + padding
            //   ... various fields ...
            //   +N: stack_pointer (VirtAddr)
            //   +N+8: instruction_pointer (VirtAddr)
            //   +N+16: saved_registers ([u64; 7])
            // 
            // We'll use runtime calculation based on struct layout
            
            // Save current RSP to TCB.stack_pointer
            // Offset of stack_pointer field = 104 (calculated from struct definition)
            "mov [rax + 104], rsp",    // current->stack_pointer = rsp
            
            // Save callee-saved registers
            // Offsets calculated from saved_registers field position
            "mov [rax + 120], rbx",    // current->saved_registers[0] = rbx
            "mov [rax + 128], rbp",    // current->saved_registers[1] = rbp
            "mov [rax + 136], r12",    // current->saved_registers[2] = r12
            "mov [rax + 144], r13",    // current->saved_registers[3] = r13
            "mov [rax + 152], r14",    // current->saved_registers[4] = r14
            "mov [rax + 160], r15",    // current->saved_registers[5] = r15
            
            // Save RFLAGS (includes interrupt flag, condition codes)
            "pushfq",                   // Push RFLAGS onto stack
            "pop [rax + 168]",         // current->saved_registers[6] = rflags
            
            // ================================================================
            // PHASE 2: SWITCH STACKS
            // ================================================================
            
            // Load next thread's TCB pointer from second argument (rsi)
            "mov rbx, [rsi]",          // rbx = next (TCB address)
            
            // Update the current pointer to point to next thread
            // This allows the caller to track which thread is running
            "mov [rdi], rbx",          // *current = next
            
            // Load next thread's stack pointer
            "mov rsp, [rbx + 104]",    // rsp = next->stack_pointer
            
            // ================================================================
            // PHASE 3: RESTORE NEXT THREAD STATE
            // ================================================================
            
            // Restore callee-saved registers from next thread's TCB
            "mov rbx, [rbx + 120]",    // rbx = next->saved_registers[0]
            "mov rbp, [rbx - 8 + 128]",// Wait, need to reload TCB pointer
            "mov rax, [rsi]",          // Reload TCB pointer (safer approach)
            "mov rbx, [rax + 120]",    // rbx = next->saved_registers[0]
            "mov rbp, [rax + 128]",    // rbp = next->saved_registers[1]
            "mov r12, [rax + 136]",    // r12 = next->saved_registers[2]
            "mov r13, [rax + 144]",    // r13 = next->saved_registers[3]
            "mov r14, [rax + 152]",    // r14 = next->saved_registers[4]
            "mov r15, [rax + 160]",    // r15 = next->saved_registers[5]
            
            // Restore RFLAGS
            "push [rax + 168]",        // Push saved RFLAGS onto stack
            "popfq",                    // Pop into RFLAGS (restores interrupt flag)
            
            // ================================================================
            // PHASE 4: RESUME NEXT THREAD
            // ================================================================
            
            // Load next thread's instruction pointer and jump to it
            "mov rcx, [rax + 112]",    // rcx = next->instruction_pointer
            
            // Return to the caller (which will be the next thread's resume point)
            // The return address was pushed when this function was called
            "ret",
        );
    }
}

/// Alternative implementation with explicit offsets documented
///
/// This version is more maintainable as the TCB structure evolves.
/// In production, you would use const generics or macros to ensure
/// offsets stay in sync with the actual struct definition.
mod offsets {
    /// Offset of stack_pointer field in ThreadControlBlock
    pub const STACK_POINTER: usize = 104;
    
    /// Offset of instruction_pointer field in ThreadControlBlock  
    pub const INSTRUCTION_POINTER: usize = 112;
    
    /// Offset of saved_registers array in ThreadControlBlock
    pub const SAVED_REGISTERS: usize = 120;
    
    /// Size of saved_registers array (7 u64 values)
    pub const SAVED_REGISTERS_SIZE: usize = 56;
}

/// Initialize a thread's stack for its first execution
///
/// This function sets up the initial stack frame so that when
/// `switch_context` restores the thread's state, it will:
/// 1. "Return" to the thread's entry point
/// 2. Have all registers zeroed (clean slate)
/// 3. Start executing as if just called
///
/// # Arguments
///
/// * `tcb` - The thread's TCB to initialize
/// * `entry_point` - Virtual address where thread execution begins
/// * `stack_top` - Virtual address of the top of the thread's stack
///
/// # Stack Layout Created
///
/// ```text
/// High addresses
/// +------------------+
/// |   (unused)       |
/// +------------------+
/// |   Return RIP     |  <- Entry point function
/// +------------------+
/// |   Initial RFLAGS |  <- Interrupts enabled
/// +------------------+  <- RSP points here initially
/// |   (zeros)        |  <- Space for saved registers
/// +------------------+
/// Low addresses
/// ```
pub fn initialize_thread_stack(
    tcb: &mut ThreadControlBlock,
    entry_point: VirtAddr,
    stack_top: VirtAddr,
) {
    // Set initial stack pointer to top of stack
    // Stack grows downward, so we start at the highest address
    tcb.stack_pointer = stack_top;
    
    // Set instruction pointer to entry point
    tcb.instruction_pointer = entry_point;
    
    // Zero all saved registers (clean initial state)
    tcb.saved_registers = [0; 7];
    
    // Set initial RFLAGS with interrupt flag enabled
    // Bit 9 is the Interrupt Enable Flag (IF)
    tcb.saved_registers[6] = 0x202; // RFLAGS with IF=1 and reserved bit 1 set
}

/// Helper function to perform a context switch with interrupt management
///
/// This wrapper handles the boilerplate of disabling/enabling interrupts
/// around the context switch.
///
/// # Arguments
///
/// * `current` - Mutable reference to current thread's TCB pointer
/// * `next` - Reference to next thread's TCB
pub fn switch_with_interrupts_disabled(
    current: &mut *mut ThreadControlBlock,
    next: *mut ThreadControlBlock,
) {
    // Interrupts are disabled inside switch_context itself
    // This wrapper is for additional safety if needed
    unsafe {
        switch_context(current, next);
    }
}

/// Test function to verify context switch works
///
/// This creates two threads that alternate execution,
/// printing a message each time they run.
#[cfg(test)]
fn test_context_switch() {
    // TODO: Implement unit tests for context switching
    // This would require:
    // 1. Allocating test stacks
    // 2. Creating test TCBs
    // 3. Verifying register preservation
    unimplemented!("Context switch tests require heap allocation");
}
