//! Minimal x86-64 interrupt/exception table for early bring-up.
//!
//! This kernel treats unexpected exceptions as fatal. PIC/APIC routing and
//! device IRQ acknowledgement are not implemented; interrupts remain disabled
//! until a complete routing/EOI path is integrated.

use alloc::boxed::Box;
use spin::Mutex;
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::println;

static IDT_READY: Mutex<bool> = Mutex::new(false);

/// Initialize and load the exception table after GDT/TSS setup.
pub fn init() {
    let mut idt = InterruptDescriptorTable::new();

    idt.divide_error.set_handler_fn(fatal_no_error);
    idt.debug.set_handler_fn(fatal_no_error);
    idt.non_maskable_interrupt.set_handler_fn(fatal_no_error);
    idt.breakpoint.set_handler_fn(fatal_no_error);
    idt.overflow.set_handler_fn(fatal_no_error);
    idt.bound_range_exceeded.set_handler_fn(fatal_no_error);
    idt.invalid_opcode.set_handler_fn(fatal_no_error);
    idt.device_not_available.set_handler_fn(fatal_no_error);

    // SAFETY: the GDT initializer creates TSS IST slot 0; IDT IST index 1
    // selects that first slot according to the x86 architectural convention.
    unsafe {
        idt.double_fault
            .set_handler_fn(fatal_double_fault)
            .set_stack_index(1);
    }

    idt.invalid_tss.set_handler_fn(fatal_with_error);
    idt.segment_not_present.set_handler_fn(fatal_with_error);
    idt.stack_segment_fault.set_handler_fn(fatal_with_error);
    idt.general_protection_fault.set_handler_fn(fatal_with_error);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.x87_floating_point.set_handler_fn(fatal_no_error);
    idt.alignment_check.set_handler_fn(fatal_with_error);
    idt.machine_check.set_handler_fn(fatal_machine_check);
    idt.simd_floating_point.set_handler_fn(fatal_no_error);
    idt.virtualization.set_handler_fn(fatal_no_error);

    let idt_ref: &'static InterruptDescriptorTable = Box::leak(Box::new(idt));
    // SAFETY: idt_ref is leaked and remains valid for the kernel lifetime.
    unsafe { idt_ref.load() };
    *IDT_READY.lock() = true;
    println!("  [IDT] CPU exception table loaded; device IRQ routing is not enabled");
}

extern "x86-interrupt" fn fatal_no_error(frame: InterruptStackFrame) {
    println!("FATAL: unhandled CPU exception");
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt();
}

extern "x86-interrupt" fn fatal_with_error(
    frame: InterruptStackFrame,
    error_code: u64,
) {
    println!("FATAL: CPU exception, error code={:#x}", error_code);
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt();
}

extern "x86-interrupt" fn fatal_double_fault(
    frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    println!("FATAL: double fault, error code={:#x}", error_code);
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt()
}

extern "x86-interrupt" fn page_fault_handler(
    frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let faulting_address = match Cr2::read() {
        Ok(address) => address,
        Err(_) => {
            println!("FATAL: CR2 contains a non-canonical address");
            fatal_halt()
        }
    };
    println!("FATAL: page fault at {:#018x}", faulting_address.as_u64());
    println!("Error flags={:#x}", error_code.bits());
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt();
}

extern "x86-interrupt" fn fatal_machine_check(frame: InterruptStackFrame) -> ! {
    println!("FATAL: machine check exception");
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt()
}

fn fatal_halt() -> ! {
    interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

/// Return whether the IDT was loaded.
#[inline]
pub fn is_initialized() -> bool {
    *IDT_READY.lock()
}
