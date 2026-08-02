//! Global Descriptor Table (GDT) and Task State Segment (TSS) Implementation
//!
//! The GDT defines memory segments for the CPU. In x86_64 long mode,
//! segmentation is mostly disabled in favor of paging, but we still need:
//! - Code segment descriptors (kernel and user mode)
//! - Data segment descriptors (kernel and user mode)
//! - Task State Segment (TSS) for interrupt stack switching
//!
//! # KeefeOS Security Architecture
//!
//! The GDT establishes the foundation for privilege separation:
//! - Kernel Code/Data: DPL=0 (Ring 0) - full hardware access
//! - User Code/Data: DPL=3 (Ring 3) - restricted, capability-based permissions
//!
//! Actual memory protection is enforced by page tables, not segments.
//! Segments primarily define privilege levels in long mode.
//!
//! # Task State Segment (TSS)
//!
//! The TSS is critical for reliable exception handling:
//! - Defines separate stacks for each privilege level (Ring 0, 1, 2)
//! - Provides Interrupt Stack Table (IST) for specific exceptions
//! - IST entries allow automatic stack switching on interrupt entry
//!
//! We use IST entry 1 for the Double Fault handler to ensure it always
//! has a valid stack, even if the main kernel stack overflows.
//!
//! # x86_64 GDT Entry Structure (8 bytes per entry)
//!
//! ```text
//! Bits 0-15:    Limit (low) - segment size
//! Bits 16-39:   Base Address (low + middle)
//! Bits 40-47:   Access Byte (type, DPL, present bit)
//! Bits 48-51:   Limit (high) + Flags
//! Bits 52-55:   Base Address (high)
//! ```
//!
//! # Access Byte Format
//!
//! ```text
//! Bit 7: Present (must be 1 for valid segments)
//! Bits 5-6: DPL (Descriptor Privilege Level: 0=kernel, 3=user)
//! Bit 4: S (Storage segment: 1=code/data, 0=system)
//! Bits 0-3: Type (executable, writable, readable, etc.)
//! ```

#![no_std]

use x86_64::structures::gdt::{
    Descriptor,
    DescriptorFlags,
    GlobalDescriptorTable,
    SegmentSelector,
};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use crate::console::println;

/// Stack size for the double fault Interrupt Stack Table (IST)
///
/// This stack must be large enough to handle deep call chains during
/// double fault diagnostics. 32KB provides ample space while keeping
/// memory usage reasonable.
const DOUBLE_FAULT_STACK_SIZE: usize = 32 * 1024;

/// Static storage for the Task State Segment
///
/// The TSS must have a 'static lifetime because the CPU holds a pointer to it.
/// Moving or dropping the TSS while the CPU references it would cause undefined behavior.
static mut TSS: Option<TaskStateSegment> = None;

/// Static storage for the double fault IST stack
///
/// We use `#[repr(align(16))]` to ensure proper stack alignment.
/// The stack is zero-initialized and will be used exclusively for
/// double fault handling.
#[repr(align(16))]
struct AlignedStack([u8; DOUBLE_FAULT_STACK_SIZE]);
static mut DOUBLE_FAULT_STACK: Option<AlignedStack> = None;

/// Wrapper for the Global Descriptor Table
///
/// This struct encapsulates the GDT and provides safe access to
/// segment selectors. The actual GDT is stored statically.
pub struct GlobalDescriptorTableWrapper {
    /// Cached kernel code segment selector
    pub kernel_code_selector: SegmentSelector,
    /// Cached kernel data segment selector
    pub kernel_data_selector: SegmentSelector,
    /// Cached user code segment selector
    pub user_code_selector: SegmentSelector,
    /// Cached user data segment selector
    pub user_data_selector: SegmentSelector,
    /// Cached TSS selector
    pub tss_selector: SegmentSelector,
}

/// Wrapper for the Task State Segment
pub struct TaskStateSegmentWrapper {
    /// Pointer to the static TSS
    pub tss: &'static mut TaskStateSegment,
}

impl GlobalDescriptorTableWrapper {
    /// Initialize the GDT with all required segment descriptors
    ///
    /// This function creates:
    /// 1. Kernel Code Segment (DPL=0, executable, readable, long mode)
    /// 2. Kernel Data Segment (DPL=0, writable)
    /// 3. User Code Segment (DPL=3, executable, readable, long mode)
    /// 4. User Data Segment (DPL=3, writable)
    /// 5. TSS Descriptor (points to our Task State Segment)
    ///
    /// # Returns
    ///
    /// A wrapper containing cached segment selectors for efficient access.
    ///
    /// # Safety
    ///
    /// This function modifies CPU state and must be called early in boot,
    /// before enabling interrupts or switching to user mode.
    pub fn init() -> Self {
        // Initialize the TSS first (required before creating TSS descriptor)
        unsafe {
            init_tss();
        }
        
        // Create and populate the GDT
        let mut gdt = GlobalDescriptorTable::new();
        
        // ================================================================
        // KERNEL SEGMENTS (Ring 0)
        // ================================================================
        
        // Kernel Code Segment
        // Flags: Present, Ring 0, Executable, Readable, Long Mode (64-bit)
        // This segment is used when the CPU is executing kernel code.
        // In long mode, the base and limit are ignored (flat memory model).
        let kernel_code_selector = gdt.add_entry(Descriptor::KernelCodeSegment(
            DescriptorFlags::KERNEL_CODE64
        ));
        
        // Kernel Data Segment
        // Flags: Present, Ring 0, Writable
        // Used for kernel data accesses. In long mode, all data segments
        // typically point to the same flat address space.
        let kernel_data_selector = gdt.add_entry(Descriptor::KernelDataSegment(
            DescriptorFlags::KERNEL_DATA
        ));
        
        // ================================================================
        // USER SEGMENTS (Ring 3)
        // ================================================================
        
        // User Code Segment
        // Flags: Present, Ring 3, Executable, Readable, Long Mode (64-bit)
        // Critical for capability-based security: user processes run with DPL=3,
        // preventing direct access to privileged instructions and memory.
        let user_code_selector = gdt.add_entry(Descriptor::UserCodeSegment(
            DescriptorFlags::USER_CODE64
        ));
        
        // User Data Segment
        // Flags: Present, Ring 3, Writable
        // User-space data accesses go through this segment.
        // Actual protection is enforced by page tables (U/S flag).
        let user_data_selector = gdt.add_entry(Descriptor::UserDataSegment(
            DescriptorFlags::USER_DATA
        ));
        
        // ================================================================
        // TASK STATE SEGMENT DESCRIPTOR
        // ================================================================
        
        // TSS Descriptor
        // Points to our static TSS structure. Required for:
        // - Stack switching on privilege level changes
        // - Interrupt Stack Table (IST) functionality
        // - I/O port permissions (not used yet)
        let tss_selector = unsafe {
            if let Some(ref tss) = TSS {
                gdt.add_entry(Descriptor::TaskStateSegment(tss))
            } else {
                panic!("TSS not initialized before GDT setup");
            }
        };
        
        // Load the GDT into the CPU
        // This makes our segment descriptors active
        gdt.load();
        
        println!("  [GDT] Initialized with {} descriptors", 5);
        println!("  [GDT] Kernel Code: selector={:#06x}, DPL=0", kernel_code_selector.0);
        println!("  [GDT] User Code:   selector={:#06x}, DPL=3", user_code_selector.0);
        println!("  [GDT] TSS loaded at: {:?}", unsafe { 
            TSS.as_ref().unwrap().interrupt_stack_table[1] 
        });
        
        GlobalDescriptorTableWrapper {
            kernel_code_selector,
            kernel_data_selector,
            user_code_selector,
            user_data_selector,
            tss_selector,
        }
    }
    
    /// Get the kernel code segment selector
    #[inline]
    pub const fn kernel_code_selector(&self) -> SegmentSelector {
        self.kernel_code_selector
    }
    
    /// Get the user code segment selector
    #[inline]
    pub const fn user_code_selector(&self) -> SegmentSelector {
        self.user_code_selector
    }
}

/// Initialize the Task State Segment (TSS)
///
/// The TSS configures:
/// 1. Stack pointers for each privilege level (we only use Ring 0)
/// 2. Interrupt Stack Table (IST) entries for specific exceptions
///
/// # Safety
///
/// This function initializes static mutable state. Must be called once
/// before the GDT is set up (since GDT references the TSS).
unsafe fn init_tss() {
    // Allocate the double fault stack
    DOUBLE_FAULT_STACK = Some(AlignedStack([0; DOUBLE_FAULT_STACK_SIZE]));
    
    // Create and configure the TSS
    let tss = &mut *(&mut TSS as *mut Option<TaskStateSegment>) as *mut Option<TaskStateSegment>;
    *tss = Some(TaskStateSegment::new());
    
    if let Some(ref mut tss) = *tss {
        // Configure IST entry 1 for double fault handling
        //
        // Why IST for double faults?
        // - Double faults often occur due to stack overflow
        // - Using the corrupted stack would cause a triple fault (reboot)
        // - IST provides a guaranteed-clean stack for recovery/diagnostics
        //
        // Stack grows downward, so we point to the TOP of the allocated region
        let stack_top = VirtAddr::new(
            (&raw mut DOUBLE_FAULT_STACK as *mut Option<AlignedStack> as u64)
                + DOUBLE_FAULT_STACK_SIZE as u64
        );
        tss.interrupt_stack_table[1] = stack_top;  // IST index 1 matches IDT set_stack_index(1)
        
        // IST entries 2-7 are available for other critical handlers:
        // - IST 2: NMI (Non-Maskable Interrupt)
        // - IST 3: Machine Check
        // - IST 4-7: Reserved for future use
        
        // Note: We don't set the Ring 0 stack pointer (rsp0) here because
        // each kernel thread will have its own stack. The scheduler will
        // update rsp0 when switching threads (future implementation).
        
        println!("  [TSS] Configured with {} byte double fault stack", DOUBLE_FAULT_STACK_SIZE);
        println!("  [TSS] IST[1] (Double Fault): {:?}", stack_top);
    }
}

/// Get a reference to the static TSS
///
/// # Panics
///
/// Panics if the TSS has not been initialized yet.
pub fn get_tss() -> &'static mut TaskStateSegment {
    unsafe {
        TSS.as_mut().expect("TSS not initialized - call GDT::init() first")
    }
}

/// Load the TSS into the CPU's Task Register
///
/// This must be called after GDT initialization to activate the TSS.
///
/// # Arguments
///
/// * `tss_selector` - The segment selector for the TSS (from GDT setup)
///
/// # Safety
///
/// Modifies CPU state. The TSS must be valid and the GDT must be loaded.
pub fn load_tss(tss_selector: SegmentSelector) {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::segmentation::load_tr;
    
    // The TSS is loaded using the load_tr instruction
    // This tells the CPU where to find the TSS for stack switching
    unsafe {
        load_tr(tss_selector);
    }
    
    println!("  [TSS] Loaded into task register (selector={:#06x})", tss_selector.0);
}

/// Complete GDT + TSS initialization
///
/// This is the main entry point that:
/// 1. Initializes the TSS with IST stacks
/// 2. Creates and loads the GDT
/// 3. Loads the TSS into the task register
///
/// # Returns
///
/// A wrapper containing all segment selectors for later use.
pub fn init() -> GlobalDescriptorTableWrapper {
    let gdt_wrapper = GlobalDescriptorTableWrapper::init();
    load_tss(gdt_wrapper.tss_selector);
    gdt_wrapper
}
