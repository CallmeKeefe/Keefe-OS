//! CPU Exception Frame and Diagnostic Utilities
//!
//! This module provides structures and utilities for capturing
//! and displaying CPU exception state.


use x86_64::structures::idt::InterruptStackFrame;

/// Captured exception frame information
///
/// This struct provides a snapshot of CPU state at the time of an exception.
/// It's used for diagnostics and logging.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ExceptionFrame {
    /// Instruction pointer (RIP) - where the exception occurred
    pub instruction_pointer: u64,
    /// Code segment (CS) - which segment was active
    pub code_segment: u64,
    /// CPU flags (RFLAGS) - condition codes, interrupt enable, etc.
    pub cpu_flags: u64,
    /// Stack pointer (RSP) - current stack position
    pub stack_pointer: u64,
    /// Stack segment (SS) - data segment for stack
    pub stack_segment: u64,
}

impl From<&InterruptStackFrame> for ExceptionFrame {
    fn from(frame: &InterruptStackFrame) -> Self {
        Self {
            instruction_pointer: frame.instruction_pointer.as_u64(),
            code_segment: frame.code_segment.0 as u64,
            cpu_flags: frame.cpu_flags.bits(),
            stack_pointer: frame.stack_pointer.as_u64(),
            stack_segment: frame.stack_segment.0 as u64,
        }
    }
}

impl ExceptionFrame {
    /// Create a new exception frame from raw values
    #[inline]
    pub const fn new(
        rip: u64,
        cs: u64,
        rflags: u64,
        rsp: u64,
        ss: u64,
    ) -> Self {
        Self {
            instruction_pointer: rip,
            code_segment: cs,
            cpu_flags: rflags,
            stack_pointer: rsp,
            stack_segment: ss,
        }
    }
    
    /// Returns true if the exception occurred in user mode (Ring 3)
    ///
    /// The RPL (Requested Privilege Level) is stored in the bottom 2 bits
    /// of the code segment selector.
    #[inline]
    pub fn is_user_mode(&self) -> bool {
        // RPL is in bits 0-1 of the segment selector
        (self.code_segment & 0b11) == 0b11
    }
    
    /// Returns the privilege level (0-3) at the time of exception
    #[inline]
    pub fn privilege_level(&self) -> u8 {
        (self.code_segment & 0b11) as u8
    }
}

/// Page fault error code flags
///
/// These flags are pushed on the stack by the CPU when a page fault occurs.
/// They describe the nature of the fault.
#[derive(Debug, Clone, Copy)]
pub struct PageFaultFlags {
    pub value: u64,
}

impl PageFaultFlags {
    /// Create from raw error code value
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Bit 0: 0 = not present, 1 = protection violation
    #[inline]
    pub fn caused_by_protection(&self) -> bool {
        self.value & 0b1 != 0
    }
    
    /// Bit 1: 0 = read, 1 = write
    #[inline]
    pub fn was_write(&self) -> bool {
        self.value & 0b10 != 0
    }
    
    /// Bit 2: 0 = kernel mode, 1 = user mode
    #[inline]
    pub fn was_user_mode(&self) -> bool {
        self.value & 0b100 != 0
    }
    
    /// Bit 3: 1 = reserved bit violation
    #[inline]
    pub fn was_reserved_bit(&self) -> bool {
        self.value & 0b1000 != 0
    }
    
    /// Bit 4: 1 = instruction fetch
    #[inline]
    pub fn was_instruction_fetch(&self) -> bool {
        self.value & 0b10000 != 0
    }
    
    /// Bit 5: 1 = protection key violation (PKU)
    #[inline]
    pub fn was_protection_key(&self) -> bool {
        self.value & 0b100000 != 0
    }
    
    /// Bit 6: 1 = shadow stack access
    #[inline]
    pub fn was_shadow_stack(&self) -> bool {
        self.value & 0b1000000 != 0
    }
    
    /// Bit 7: 1 = HLAT (Host Address Linear Translation)
    #[inline]
    pub fn was_hlat(&self) -> bool {
        self.value & 0b10000000 != 0
    }
    
    /// Bits 15-8: SGX-related (Software Guard Extensions)
    #[inline]
    pub fn sgx_error(&self) -> u8 {
        ((self.value >> 8) & 0xFF) as u8
    }
}

/// General Protection Fault error code decoder
#[derive(Debug, Clone, Copy)]
pub struct GPErrorCode {
    pub value: u64,
}

impl GPErrorCode {
    /// Create from raw error code
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Returns true if this is an external event (not caused by loading a selector)
    #[inline]
    pub fn is_external(&self) -> bool {
        self.value & 0b1 != 0
    }
    
    /// Extract the selector index from the error code
    ///
    /// For exceptions caused by loading a segment descriptor,
    /// bits 15-3 contain the selector index.
    #[inline]
    pub fn selector_index(&self) -> u16 {
        ((self.value >> 1) & 0xFFFF) as u16
    }
    
    /// Extract the table type from the error code
    ///
    /// Bits 2-1 indicate which table the selector references:
    /// - 0b00: GDT (Global Descriptor Table)
    /// - 0b01: IDT (Interrupt Descriptor Table)
    /// - 0b10: LDT (Local Descriptor Table)
    /// - 0b11: IDT
    #[inline]
    pub fn table_type(&self) -> &'static str {
        match (self.value >> 2) & 0b11 {
            0 => "GDT",
            1 => "IDT",
            2 => "LDT",
            3 => "IDT",
            _ => unreachable!(),
        }
    }
}

/// Double Fault Error Code
///
/// Double faults always push an error code of 0.
/// The presence of a double fault is the important signal,
/// not the error code value.
#[derive(Debug, Clone, Copy)]
pub struct DoubleFaultCode {
    pub value: u64,
}

impl DoubleFaultCode {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Double faults always have error code 0
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.value == 0
    }
}

/// Invalid TSS Error Code decoder
#[derive(Debug, Clone, Copy)]
pub struct TSSErrorCode {
    pub value: u64,
}

impl TSSErrorCode {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Returns true if the error was caused by an external event
    #[inline]
    pub fn is_external(&self) -> bool {
        self.value & 0b1 != 0
    }
    
    /// Extract the selector index
    #[inline]
    pub fn selector_index(&self) -> u16 {
        ((self.value >> 1) & 0xFFFF) as u16
    }
}

/// Alignment Check Error Code decoder
#[derive(Debug, Clone, Copy)]
pub struct ACErrorCode {
    pub value: u64,
}

impl ACErrorCode {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Alignment check errors always have error code 0
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.value == 0
    }
}

/// Segment Not Present Error Code decoder
#[derive(Debug, Clone, Copy)]
pub struct NPErrorCode {
    pub value: u64,
}

impl NPErrorCode {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Returns true if the error was caused by an external event
    #[inline]
    pub fn is_external(&self) -> bool {
        self.value & 0b1 != 0
    }
    
    /// Extract the selector index
    #[inline]
    pub fn selector_index(&self) -> u16 {
        ((self.value >> 1) & 0xFFFF) as u16
    }
    
    /// Extract the table type
    #[inline]
    pub fn table_type(&self) -> &'static str {
        match (self.value >> 2) & 0b11 {
            0 => "GDT",
            1 => "IDT",
            2 => "LDT",
            3 => "IDT",
            _ => unreachable!(),
        }
    }
}

/// Stack-Segment Fault Error Code decoder
#[derive(Debug, Clone, Copy)]
pub struct SSErrorCode {
    pub value: u64,
}

impl SSErrorCode {
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
    
    /// Returns true if the error was caused by an external event
    #[inline]
    pub fn is_external(&self) -> bool {
        self.value & 0b1 != 0
    }
    
    /// Extract the selector index
    #[inline]
    pub fn selector_index(&self) -> u16 {
        ((self.value >> 1) & 0xFFFF) as u16
    }
}
