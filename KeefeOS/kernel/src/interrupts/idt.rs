//! Minimal x86-64 interrupt/exception table for early bring-up.
//!
//! All unexpected vectors are fatal during this development phase. The
//! double-fault handler uses a dedicated TSS IST stack. Device IRQs are not
//! considered enabled: PIC/APIC programming and EOI delivery are still TODO.

use alloc::boxed::Box;
use spin::Mutex;
use x86_64::instructions::interrupts;
use x86_64::registers::control::Cr2;
use x86_64::set_general_handler;
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};
use x86_64::VirtAddr;

use crate::println;

static IDT_READY: Mutex<bool> = Mutex::new(false);

/// Initialize and load the interrupt descriptor table.
///
/// This must run after the GDT/TSS setup and before enabling interrupts.
pub fn init() {
    let mut idt = InterruptDescriptorTable::new();

    // Install a compatible generic handler before overriding the two handlers
    // whose hardware-defined signatures must be non-returning.
    set_general_handler!(&mut idt, unexpected_interrupt_handler);

    // SAFETY: GDT/TSS initialization creates TSS IST entry 1 before this call.
    unsafe {
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(1);
        idt.page_fault.set_handler_fn(page_fault_handler);
    }

    // Leak the IDT because the CPU keeps a pointer to it after lidt.
    let idt_ref: &'static InterruptDescriptorTable = Box::leak(Box::new(idt));
    unsafe { idt_ref.load() };
    *IDT_READY.lock() = true;

    println!("  [IDT] Loaded generic fatal handlers and dedicated double-fault/page-fault handlers");
}

/// Generic handler used for unexpected exceptions and unconfigured IRQ vectors.
extern "x86-interrupt" fn unexpected_interrupt_handler(
    frame: InterruptStackFrame,
    vector: u8,
    error_code: Option<u64>,
) {
    println!("FATAL: unexpected interrupt/exception vector {}", vector);
    if let Some(code) = error_code {
        println!("Error code: {:#x}", code);
    }
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt()
}

/// Double faults are non-recoverable in the current kernel.
extern "x86-interrupt" fn double_fault_handler(
    frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    println!("FATAL: double fault; error code={:#x}", error_code);
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt()
}

/// Page faults are diagnostic-only; there is no demand pager or process isolation yet.
extern "x86-interrupt" fn page_fault_handler(
    frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) -> ! {
    let faulting_address = match Cr2::read() {
        Ok(address) => address,
        Err(_) => {
            println!("FATAL: page fault reported a non-canonical CR2 address");
            fatal_halt()
        }
    };

    println!("FATAL: page fault at {:#018x}", faulting_address.as_u64());
    println!("Error flags: {:#x}", error_code.bits());
    println!("RIP={:#018x}", frame.instruction_pointer.as_u64());
    fatal_halt()
}

fn fatal_halt() -> ! {
    interrupts::disable();
    loop {
        x86_64::instructions::hlt();
    }
}

/// Report whether the IDT has been loaded.
#[inline]
pub fn is_initialized() -> bool {
    *IDT_READY.lock()
}
