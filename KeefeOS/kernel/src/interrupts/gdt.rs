//! x86-64 Global Descriptor Table and Task State Segment setup.
//!
//! The GDT and TSS are heap-leaked so the CPU never retains pointers to
//! short-lived descriptors or stack storage. This initial implementation
//! configures kernel/user segment selectors and an IST stack for double fault.
//! It does not implement user-mode entry or per-thread RSP0 management.

use alloc::boxed::Box;
use x86_64::instructions::tables::load_tss;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::println;

const DOUBLE_FAULT_STACK_SIZE: usize = 32 * 1024;

#[repr(align(16))]
struct AlignedStack([u8; DOUBLE_FAULT_STACK_SIZE]);

/// Selector bundle returned by GDT initialization.
pub struct GlobalDescriptorTableWrapper {
    /// Kernel code selector.
    pub kernel_code_selector: SegmentSelector,
    /// Kernel data selector.
    pub kernel_data_selector: SegmentSelector,
    /// User code selector (not proof that user mode is implemented).
    pub user_code_selector: SegmentSelector,
    /// User data selector (not proof that user mode is implemented).
    pub user_data_selector: SegmentSelector,
    /// Task-state-segment selector.
    pub tss_selector: SegmentSelector,
}

/// Holder type for callers that need a TSS reference in later work.
pub struct TaskStateSegmentWrapper {
    /// Statically lived TSS reference.
    pub tss: &'static mut TaskStateSegment,
}

impl GlobalDescriptorTableWrapper {
    fn init() -> Self {
        // Both the TSS and its IST stack must remain at stable addresses for
        // the entire time the loaded GDT/TSS is active.
        let double_fault_stack = Box::leak(Box::new(AlignedStack([0; DOUBLE_FAULT_STACK_SIZE])));
        let stack_top = VirtAddr::new(
            double_fault_stack.0.as_ptr() as u64 + DOUBLE_FAULT_STACK_SIZE as u64,
        );

        let mut tss_value = TaskStateSegment::new();
        // IDT IST index 1 selects TSS array entry 0 (the array is zero-indexed).
        tss_value.interrupt_stack_table[0] = stack_top;
        let tss: &'static mut TaskStateSegment = Box::leak(Box::new(tss_value));

        let mut gdt_value = GlobalDescriptorTable::new();
        let kernel_code_selector = gdt_value.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt_value.append(Descriptor::kernel_data_segment());
        let user_code_selector = gdt_value.append(Descriptor::user_code_segment());
        let user_data_selector = gdt_value.append(Descriptor::user_data_segment());
        let tss_selector = gdt_value.append(Descriptor::tss_segment(&*tss));

        // The CPU retains the GDT base after load, so leak the table as well.
        let gdt: &'static mut GlobalDescriptorTable = Box::leak(Box::new(gdt_value));
        gdt.load();

        // SAFETY: tss_selector was generated from a valid TSS descriptor in
        // the GDT just loaded, and the TSS and IST stack both live forever.
        unsafe { load_tss(tss_selector) };

        println!("  [GDT] Initialized kernel/user selectors and TSS");
        println!("  [GDT] Double fault IST stack top: {:#018x}", stack_top.as_u64());

        Self {
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector,
        }
    }

    /// Return the kernel code selector.
    #[inline]
    pub const fn kernel_code_selector(&self) -> SegmentSelector {
        self.kernel_code_selector
    }

    /// Return the user code selector.
    #[inline]
    pub const fn user_code_selector(&self) -> SegmentSelector {
        self.user_code_selector
    }
}

/// Initialize and load the GDT and TSS once during early boot.
pub fn init() -> GlobalDescriptorTableWrapper {
    GlobalDescriptorTableWrapper::init()
}
