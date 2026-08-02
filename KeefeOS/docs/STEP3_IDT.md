# KeefeOS Step 3: CPU Exceptions & Interrupt Descriptor Table (IDT)

## Overview

Step 3 implements the complete interrupt and exception handling infrastructure for KeefeOS. This is critical for system stability - without proper exception handling, any fault (division by zero, page fault, etc.) would cause a triple fault and immediate system reboot.

## Architecture

### Interrupt Descriptor Table (IDT)

The IDT is the CPU's lookup table for handling interrupts and exceptions. Each of the 256 entries contains a pointer to an handler function.

**Vector Layout:**
```
Vectors 0-31:   CPU Exceptions (non-maskable)
Vectors 32-47:  Hardware IRQs (8259 PIC remapped)
Vectors 48+:    Software interrupts, syscalls, APIC
```

### Global Descriptor Table (GDT)

While x86_64 uses a flat memory model, the GDT is still required for:
- Defining privilege levels (Ring 0 vs Ring 3)
- Task State Segment (TSS) descriptor

### Task State Segment (TSS)

The TSS enables reliable exception handling through:
- **Interrupt Stack Table (IST)**: Dedicated stacks for critical handlers
- **Double Fault Protection**: IST entry 1 provides a clean stack even if the main stack overflows

## Files Created

### `/kernel/src/interrupts/mod.rs`
Main module entry point with:
- Hardware interrupt offset configuration (0x20)
- IRQ to vector mapping utilities
- Module exports

### `/kernel/src/interrupts/idt.rs`
Complete IDT implementation:
- 22 CPU exception handlers (vectors 0-31)
- 16 hardware IRQ placeholders (vectors 32-47)
- Specialized handlers for:
  - **Division Error (#DE)**: Vector 0
  - **Page Fault (#PF)**: Vector 14 (reads CR2 register)
  - **General Protection Fault (#GP)**: Vector 13
  - **Double Fault (#DF)**: Vector 8 (uses IST)
- Macro-generated stub handlers for code deduplication

### `/kernel/src/interrupts/gdt.rs`
GDT and TSS implementation:
- Kernel Code/Data segments (DPL=0)
- User Code/Data segments (DPL=3)
- TSS with 32KB double fault stack
- IST configuration for stack switching

### `/kernel/src/interrupts/exceptions.rs`
Exception frame utilities:
- `ExceptionFrame`: Captured CPU state snapshot
- Error code decoders for:
  - Page Fault flags
  - General Protection Fault
  - Invalid TSS
  - Segment Not Present
  - Stack-Segment Fault

## Security Design

### Capability-Based Exception Handling

| Privilege Level | Exception Response |
|----------------|-------------------|
| Ring 0 (Kernel) | Controlled panic → system halt |
| Ring 3 (User) | Process termination → IPC to crash handler (future) |

### Key Security Features

1. **IDT Protection**: Only kernel can modify IDT entries
2. **Privilege Separation**: GDT defines Ring 0 vs Ring 3
3. **Stack Isolation**: Double fault handler uses separate IST stack
4. **Error Diagnostics**: Detailed fault information for debugging

## Technical Details

### Double Fault Prevention

A double fault occurs when:
1. An exception happens while handling another exception
2. Common cause: stack overflow during exception handling

**Solution:**
```rust
// IDT entry configured with IST index 1
idt.double_fault
    .set_handler_fn(double_fault_handler)
    .set_stack_index(1); // Use dedicated IST stack
```

### Page Fault Handler

Reads the CR2 register to get the faulting address:
```rust
let faulting_addr = Cr2::read();
```

Decodes error code flags:
- Bit 0: Protection violation vs not-present
- Bit 1: Write access
- Bit 2: User mode access
- Bit 3: Reserved bit violation
- Bit 4: Instruction fetch

### Hardware Interrupt Remapping

Legacy 8259 PIC maps IRQs to vectors 0-15, colliding with CPU exceptions. We remap:
```
IRQ 0 (Timer)   → Vector 32 (0x20)
IRQ 1 (Keyboard) → Vector 33 (0x21)
...
IRQ 15          → Vector 47 (0x2F)
```

## Integration with Main Kernel

Updated `kernel/src/main.rs`:
```rust
// Initialize GDT and TSS first (required for IDT)
let gdt_wrapper = interrupts::gdt::init();

// Then initialize IDT
interrupts::idt::init();
```

## Testing

To test exception handling in QEMU:
```bash
# Trigger division by zero (will be implemented in test code)
# Expected: Controlled panic with diagnostic output

# Trigger page fault (access unmapped memory)
# Expected: Page fault handler prints CR2 address and halts
```

## Next Steps (Step 4: Multitasking)

With exceptions handled, we can now implement:
1. **PIC/APIC Initialization**: Properly acknowledge hardware interrupts
2. **Timer Interrupt**: Periodic interrupt for preemption
3. **Thread Scheduler**: Round-robin or priority-based scheduling
4. **Context Switching**: Save/restore CPU state between threads
5. **User Mode Execution**: First Ring 3 process

## Performance Considerations

| Metric | Value |
|--------|-------|
| IDT Entry Size | 16 bytes |
| Total IDT Size | 4 KB (256 entries) |
| Exception Handler Overhead | ~50-100 cycles |
| Double Fault Stack | 32 KB reserved |
| GDT Entries | 5 descriptors |

## Compliance with Requirements

✅ **Pure Rust Implementation**: All code in safe Rust with isolated unsafe blocks
✅ **Microkernel Design**: Minimal kernel, drivers will run in user-space
✅ **Capability-Based**: Privilege levels enforced via GDT/page tables
✅ **Double Fault Protection**: IST-enabled handler
✅ **Page Fault Diagnostics**: CR2 reading and error code decoding
✅ **Hardware IRQ Groundwork**: Remapping prepared for timer/keyboard
✅ **Exhaustive Documentation**: Inline comments explain all decisions

---

*Ready for Step 4: Multitasking & Timers upon approval.*
