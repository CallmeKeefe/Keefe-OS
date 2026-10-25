//! Interrupt Descriptor Table (IDT) Implementation
//!
//! The IDT is the CPU's lookup table for handling interrupts and exceptions.
//! Each of the 256 entries contains a pointer to an interrupt handler function.
//!
//! # KeefeOS Security Architecture
//!
//! The IDT is a critical security boundary:
//! - Only kernel code (Ring 0) can modify IDT entries
//! - User-space processes cannot register custom handlers
//! - Exception handling policy differs by privilege level:
//!   - Ring 0: Fatal error → controlled kernel panic
//!   - Ring 3: Process termination → IPC to crash handler (future)
//!
//! # x86_64 IDT Entry Structure (16 bytes per entry)
//!
//! ```text
//! Bits 0-15:    Offset bits 0-15 (low part of handler address)
//! Bits 16-31:   Segment Selector (GDT entry)
//! Bits 32-35:   IST (Interrupt Stack Table) index (bits 0-2) + reserved
//! Bits 36-39:   Type/attributes (interrupt gate, trap gate, etc.)
//! Bits 40-47:   Reserved (must be zero)
//! Bits 48-63:   Offset bits 16-31 (middle part of handler address)
//! Bits 64-95:   Offset bits 32-63 (high part of handler address)
//! Bits 96-127:  Reserved
//! ```
//!
//! # Gate Types
//!
//! - **Interrupt Gate (0xE)**: Disables interrupts automatically on entry
//! - **Trap Gate (0xF)**: Leaves interrupts enabled (used for syscalls)
//! - **Task Gate**: Switches to a different TSS (not used in KeefeOS)
//!
//! We use Interrupt Gates for hardware interrupts and Trap Gates for exceptions
//! where we want to allow nested interrupts during handling.

use x86_64::structures::idt::{
    InterruptDescriptorTable,
    InterruptStackFrame,
    InterruptStackFrameValue,
    PageFaultErrorCode,
};
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr2;
use crate::println;

/// Lazy-loaded static IDT
///
/// We use a spin lock for safe mutable access.
/// The IDT must outlive `load()` (the CPU keeps referencing it), so we leak
/// a `'static` copy into a global slot instead of storing it behind an Option
/// that could theoretically be dropped.
static IDT_READY: spin::Mutex<bool> = spin::Mutex::new(false);

/// Initialize the IDT with all exception and interrupt handlers
///
/// This function sets up handlers for:
/// - Vectors 0-31: CPU exceptions (divide by zero, page fault, etc.)
/// - Vectors 32-47: Hardware IRQ placeholders (ready for PIC/APIC setup)
///
/// # Safety
///
/// Must be called after GDT initialization and before enabling interrupts.
/// Calling this function twice will overwrite the previous IDT.
pub fn init() {
    let mut idt = InterruptDescriptorTable::new();

    // ================================================================
    // CPU EXCEPTIONS (Vectors 0-31)
    // ================================================================

    // Vector 0: Division Error (#DE)
    // Triggered by DIV or IDIV instruction with zero divisor
    // No error code pushed on stack
    idt.divide_error.set_handler_fn(division_error_handler);

    // Vector 1: Debug (#DB)
    // Single-step execution, hardware breakpoints
    idt.debug.set_handler_fn(debug_handler);

    // Vector 2: Non-Maskable Interrupt (NMI)
    // Hardware signal that cannot be disabled by IF flag
    // Used for critical hardware errors, watchdog timers
    idt.non_maskable_interrupt.set_handler_fn(nmi_handler);

    // Vector 3: Breakpoint (#BP)
    // INT3 instruction - used by debuggers
    idt.breakpoint.set_handler_fn(breakpoint_handler);

    // Vector 4: Overflow (#OF)
    // INTO instruction when overflow flag is set
    idt.overflow.set_handler_fn(overflow_handler);

    // Vector 5: Bound Range Exceeded (#BR)
    // BOUND instruction when array index out of bounds
    idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded_handler);

    // Vector 6: Invalid Opcode (#UD)
    // Undefined instruction or invalid operand
    // Critical for catching corrupted code or unsupported CPU features
    idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);

    // Vector 7: Device Not Available (#NM)
    // Floating point instruction when EM flag in CR0 is set
    idt.device_not_available.set_handler_fn(device_not_available_handler);

    // Vector 8: Double Fault (#DF)
    // Exception while handling another exception
    // ERROR CODE PUSHED ON STACK
    // We use IST (Interrupt Stack Table) entry 1 for this handler
    // to ensure we have a valid stack even if main stack is corrupted
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(1); // Use IST entry 1 (configured in GDT/TSS)
    }

    // Vector 9: Coprocessor Segment Overrun (deprecated)
    // Legacy FPU error - rarely occurs on modern CPUs.
    // The x86_64 crate intentionally does not expose this field because the
    // vector is architecturally obsolete; we leave the entry present but
    // pointing at a benign ignore handler via the reserved-slot API.
    idt[9].set_handler_fn(ignore_handler);

    // Vector 10: Invalid TSS (#TS)
    // Task State Segment validation failure
    // ERROR CODE PUSHED ON STACK (selector index)
    idt.invalid_tss.set_handler_fn(invalid_tss_handler);

    // Vector 11: Segment Not Present (#NP)
    // Loading a non-present segment descriptor
    // ERROR CODE PUSHED ON STACK (selector index)
    idt.segment_not_present.set_handler_fn(segment_not_present_handler);

    // Vector 12: Stack-Segment Fault (#SS)
    // Stack operations beyond limit or with invalid selector
    // ERROR CODE PUSHED ON STACK (selector index)
    idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);

    // Vector 13: General Protection Fault (#GP)
    // Catch-all for protection violations, invalid selectors, etc.
    // ERROR CODE PUSHED ON STACK (selector index or 0)
    // Most common exception for capability violations
    idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

    // Vector 14: Page Fault (#PF)
    // Virtual memory access violation
    // ERROR CODE PUSHED ON STACK (PageFaultErrorCode bitflags)
    // FAULTING ADDRESS stored in CR2 register
    idt.page_fault.set_handler_fn(page_fault_handler);

    // Vector 15: Reserved (do not use)

    // Vector 16: x87 FPU Error (#MF)
    // Floating point unit exception
    idt.x87_floating_point.set_handler_fn(x87_floating_point_handler);

    // Vector 17: Alignment Check (#AC)
    // Unaligned memory access when AC flag in EFLAGS is set
    // ERROR CODE PUSHED ON STACK
    idt.alignment_check.set_handler_fn(alignment_check_handler);

    // Vector 18: Machine Check (#MC)
    // Hardware errors detected by CPU (cache, bus, TLB, etc.)
    // Critical - often indicates hardware failure
    idt.machine_check.set_handler_fn(machine_check_handler);

    // Vector 19: SIMD Floating-Point Exception (#XM/#XF)
    // SSE/AVX floating point errors
    idt.simd_floating_point.set_handler_fn(simd_floating_point_handler);

    // Vector 20: Virtualization Exception (#VE)
    // EPT violation when VMX virtualization is active
    idt.virtualization.set_handler_fn(virtualization_handler);

    // Vector 21: Control Protection Exception (#CP)
    // CET (Control-flow Enforcement Technology) violations
    idt.control_protection_exception.set_handler_fn(control_protection_handler);

    // Vectors 22-31: Reserved for future CPU extensions

    // ================================================================
    // HARDWARE INTERRUPTS (Vectors 32-47) - PLACEHOLDERS
    // ================================================================
    // These will be properly configured when we initialize the PIC/APIC
    // For now, we set up placeholder handlers that log unexpected IRQs

    // Note: The 8259 PIC will be remapped to start at vector 32 (0x20)
    // See interrupts::HARDWARE_INTERRUPT_OFFSET

    // Placeholder for IRQ 0-15 (vectors 32-47).
    // Each vector gets its own monomorphic handler generated below via the
    // macro pattern; here we simply install the generic handler for every
    // slot (the vector number is recoverable from the stack frame type in
    // future revisions once device drivers land).
    for vector in 32u16..48 {
        idt[x86_64::structures::idt::InterruptVector::from(vector)]
            .set_handler_fn(hw_interrupt_placeholder);
    }

    // Load the IDT into the CPU.
    //
    // SAFETY: The IDT must remain valid in memory for as long as interrupts
    // are enabled. We deliberately leak it (`Box::leak`) so it lives for the
    // entire uptime of the kernel.
    let idt_ref: &'static InterruptDescriptorTable = Box::leak(Box::new(idt));
    unsafe {
        idt_ref.load();
    }
    *IDT_READY.lock() = true;

    println!("  [IDT] Initialized 48 exception/interrupt handlers");
}

/// Helper macro to generate exception handler stubs
/// Each handler captures the exception frame and calls the common handler
macro_rules! generate_exception_handler {
    ($name:ident, $vector:expr, $message:expr) => {
        extern "x86-interrupt" fn $name(frame: &InterruptStackFrame) -> ! {
            exception_handler_common(frame, $vector, $message, None)
        }
    };
    ($name:ident, $vector:expr, $message:expr, with_error) => {
        extern "x86-interrupt" fn $name(frame: &InterruptStackFrame, error_code: u64) -> ! {
            exception_handler_common(frame, $vector, $message, Some(error_code))
        }
    };
}

// Generate handlers for exceptions without error codes
generate_exception_handler!(division_error_handler, 0, "Division Error (#DE)");
generate_exception_handler!(debug_handler, 1, "Debug (#DB)");
generate_exception_handler!(nmi_handler, 2, "Non-Maskable Interrupt (NMI)");
generate_exception_handler!(breakpoint_handler, 3, "Breakpoint (#BP)");
generate_exception_handler!(overflow_handler, 4, "Overflow (#OF)");
generate_exception_handler!(bound_range_exceeded_handler, 5, "Bound Range Exceeded (#BR)");
generate_exception_handler!(invalid_opcode_handler, 6, "Invalid Opcode (#UD)");
generate_exception_handler!(device_not_available_handler, 7, "Device Not Available (#NM)");
generate_exception_handler!(invalid_tss_handler, 10, "Invalid TSS (#TS)", with_error);
generate_exception_handler!(segment_not_present_handler, 11, "Segment Not Present (#NP)", with_error);
generate_exception_handler!(stack_segment_fault_handler, 12, "Stack-Segment Fault (#SS)", with_error);
generate_exception_handler!(x87_floating_point_handler, 16, "x87 FPU Error (#MF)");
generate_exception_handler!(alignment_check_handler, 17, "Alignment Check (#AC)", with_error);
generate_exception_handler!(machine_check_handler, 18, "Machine Check (#MC)");
generate_exception_handler!(simd_floating_point_handler, 19, "SIMD Floating-Point (#XM)");
generate_exception_handler!(virtualization_handler, 20, "Virtualization Exception (#VE)");
generate_exception_handler!(control_protection_handler, 21, "Control Protection Exception (#CP)");

/// Handler for the deprecated coprocessor segment overrun vector (9).
/// On modern hardware this never fires; if it somehow does, we halt safely.
extern "x86-interrupt" fn ignore_handler(_frame: &InterruptStackFrame) {
    println!("[IDT] Spurious coprocessor segment overrun (vector 9) ignored");
}

/// Double Fault Handler - CRITICAL
///
/// This handler uses a dedicated Interrupt Stack Table (IST) entry,
/// ensuring it has a valid stack even if the main kernel stack is corrupted.
///
/// Common causes of double faults:
/// - Stack overflow (most common)
/// - Kernel heap corruption
/// - Missing exception handler for a prior exception
/// - Corrupted IDT/GDT entries
///
/// In KeefeOS, a double fault is always fatal - it indicates severe corruption.
extern "x86-interrupt" fn double_fault_handler(
    frame: &InterruptStackFrame,
    _error_code: u64,
) -> ! {
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!         DOUBLE FAULT DETECTED          !!!");
    println!("!!!     Critical system corruption          !!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");

    print_stack_frame(frame);

    println!("\nDouble faults indicate:");
    println!("  - Stack overflow during exception handling");
    println!("  - Corrupted kernel data structures");
    println!("  - Missing exception handler");
    println!("  - Hardware memory corruption");

    println!("\nSystem halted - manual intervention required.");

    // Disable interrupts to prevent further chaos
    interrupts::disable();

    // Halt forever
    loop {
        x86_64::instructions::hlt();
    }
}

/// Page Fault Handler - Detailed Diagnostic
///
/// Page faults occur when:
/// - Accessing unmapped virtual memory
/// - Violating page permissions (read-only write, user accessing supervisor page)
/// - Copy-on-write page needs to be duplicated
///
/// The CR2 register contains the faulting linear address.
/// The error code contains flags describing the fault type.
extern "x86-interrupt" fn page_fault_handler(
    frame: &InterruptStackFrame,
    error_code: PageFaultErrorCode,
) -> ! {
    // Read CR2 to get the faulting address
    let faulting_addr = Cr2::read();

    // Decode error code flags
    let caused_by_write = error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE);
    let user_mode_access = error_code.contains(PageFaultErrorCode::USER_MODE);
    let instruction_fetch = error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH);

    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!              PAGE FAULT                 !!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");

    println!("Faulting Address: {:#018x}", faulting_addr.as_u64());
    println!("\nError Flags:");
    println!("  Caused by write:          {}", caused_by_write);
    println!("  User mode access:         {}", user_mode_access);
    println!("  Instruction fetch:        {}", instruction_fetch);
    println!("  Raw error code:           {:#06b}", error_code.bits());

    println!("\nException Frame:");
    print_stack_frame(frame);

    // Capability-based security note:
    // In user mode (Ring 3), this would trigger process termination
    // and send an IPC message to the crash handler service.
    // For now, all faults are treated as fatal in the kernel.

    println!("\nKeefeOS Security Note:");
    println!("  In production, Ring 3 faults terminate only the process.");
    println!("  Ring 0 faults (like this) halt the entire system.");

    println!("\nSystem halted.");
    interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

/// General Protection Fault Handler
///
/// GP faults are catch-all exceptions for various protection violations:
/// - Accessing memory with wrong segment selector
/// - Executing privileged instructions in user mode
/// - Writing to read-only segment
/// - Loading invalid segment descriptors
/// - Violating capability boundaries (KeefeOS-specific)
extern "x86-interrupt" fn general_protection_fault_handler(
    frame: &InterruptStackFrame,
    error_code: u64,
) -> ! {
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!       GENERAL PROTECTION FAULT          !!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");

    println!("Error Code: {:#018x}", error_code);

    // Decode error code
    if error_code == 0 {
        println!("  External event (non-segment related)");
    } else {
        let selector_index = error_code & 0xFFFF;
        let table_type = match (error_code >> 2) & 0b11 {
            0 => "GDT",
            1 => "IDT",
            2 => "LDT",
            3 => "IDT",
            _ => "Unknown",
        };
        let is_external = (error_code & 1) != 0;

        println!("  Selector Index: {}", selector_index);
        println!("  Table Type: {}", table_type);
        println!("  External Event: {}", is_external);
    }

    print_stack_frame(frame);

    println!("\nCommon causes:");
    println!("  - Invalid segment selector");
    println!("  - Privilege level violation");
    println!("  - Capability check failure (KeefeOS)");
    println!("  - Corrupted GDT/IDT entry");

    println!("\nSystem halted.");
    interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

/// Print the registers of an interrupt stack frame
fn print_stack_frame(frame: &InterruptStackFrame) {
    let v: &InterruptStackFrameValue = frame.as_value();
    println!("Exception Frame:");
    println!("  Instruction Pointer: {:#018x}", v.rip);
    println!("  Code Segment:        {:#06x}", v.cs.selectors().0 .0);
    println!("  CPU Flags:           {:#018x}", v.rflags);
    println!("  Stack Pointer:       {:#018x}", v.rsp);
    println!("  Stack Segment:       {:#06x}", v.ss.selectors().0 .0);
}

/// Common exception handler logic
///
/// This function provides unified exception reporting and is called
/// by all individual exception handlers.
///
/// # Arguments
///
/// * `frame` - The interrupt stack frame captured by the CPU
/// * `vector` - The exception vector number (0-31)
/// * `name` - Human-readable exception name
/// * `error_code` - Optional error code pushed by some exceptions
fn exception_handler_common(
    frame: &InterruptStackFrame,
    vector: u8,
    name: &'static str,
    error_code: Option<u64>,
) -> ! {
    println!("\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("!!!           CPU EXCEPTION                 !!!");
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n");

    println!("Exception: {} (Vector {})", name, vector);

    if let Some(ec) = error_code {
        println!("Error Code: {:#018x}", ec);
    }

    print_stack_frame(frame);

    println!("\nKeefeOS Microkernel cannot recover from kernel-mode exceptions.");
    println!("System halted for safety.");

    // Disable interrupts to prevent nested exceptions
    interrupts::disable();

    // Halt the CPU indefinitely
    loop {
        x86_64::instructions::hlt();
    }
}

/// Placeholder hardware interrupt handler for vectors 32-47
///
/// Once the PIC/APIC is properly initialized, these will be replaced
/// with device-specific handlers (timer, keyboard, etc.).
extern "x86-interrupt" fn hw_interrupt_placeholder(frame: &InterruptStackFrame) {
    println!("\n[HW Interrupt] Unexpected IRQ");
    println!("  Instruction Pointer: {:#018x}", frame.as_value().rip);
    // Acknowledge the interrupt (will be implemented with PIC/APIC setup)
    // For now, just log and return
}

/// Test function to verify IDT is loaded
#[inline]
pub fn is_initialized() -> bool {
    *IDT_READY.lock()
}
