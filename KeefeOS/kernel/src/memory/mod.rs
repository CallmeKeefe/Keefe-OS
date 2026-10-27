//! Memory Management Subsystem
//!
//! This module provides the core memory management functionality for KeefeOS:
//! - Physical frame allocation (bitmap-based)
//! - Virtual memory management (page tables)
//! - Memory map parsing from UEFI
//!
//! # Design Decisions
//!
//! ## Bitmap vs Stack Allocator
//!
//! We chose a **Bitmap Allocator** for the following reasons:
//!
//! 1. **Compact tracking**: One bit per 4 KiB frame; bitmap size is proportional
//!    to the managed physical-address span, capped at 64 GiB.
//! 2. **Bounded bit updates**: Marking one frame allocated or free is O(1).
//! 3. **First-fit scanning**: Single-frame allocation scans bitmap words; it is
//!    O(number of tracked words), not O(1).
//! 4. **Contiguous allocation**: The current implementation performs a linear
//!    first-fit scan and is intended for early bring-up, not production workloads.
//! 5. **Future work**: Per-CPU or NUMA-aware allocators require a verified SMP
//!    model; the current global spinlock is not a NUMA/multi-core design.
//!
//! A stack allocator would be faster for LIFO patterns but suffers from:
//! - O(n) worst-case for finding free frames
//! - Poor fragmentation handling
//! - Not suitable for long-running systems
//!
//! ## Lock-Free Design
//!
//! For a single-core initial implementation, we use interior mutability
//! with `spin::Mutex`. For multi-core, we'll implement:
//! - Per-CPU allocators with work stealing
//! - Lock-free atomic bitmap operations

pub mod frame_allocator;

// The current UEFI boot path retains the firmware's active address space.
// The duplicate hand-written paging managers are not safe to initialize yet;
// keep a visible no-op until they are consolidated and tested.
#[cfg(feature = "experimental-paging")]
pub mod paging;

#[cfg(not(feature = "experimental-paging"))]
pub mod paging {
    /// Paging setup is intentionally disabled until the boot handoff and page
    /// table implementation are reconciled. This does not create page tables.
    pub fn init() {
        log::warn!("Virtual paging setup is disabled; firmware mappings are retained");
    }
}

use self::frame_allocator::{PhysicalFrameAllocator, FRAME_SIZE};
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;

/// Global physical frame allocator
/// This is initialized once during boot and used throughout the kernel lifetime
static FRAME_ALLOCATOR: Mutex<Option<PhysicalFrameAllocator>> = Mutex::new(None);

/// Prevent a second initialization from making live physical frames allocatable again.
static MEMORY_INIT_STARTED: AtomicBool = AtomicBool::new(false);

/// Initialize the memory management subsystem once during early boot.
///
/// Re-initialization is refused so existing frame ownership cannot be reset.
///
/// # Arguments
///
/// * `memory_map` - UEFI memory map indicating usable and reserved regions
/// * `kernel_start` - Physical start address of the kernel
/// * `kernel_end` - Physical end address of the kernel
///
/// Initializes memory state once. A duplicate call is rejected because replacing
/// the allocator could make still-live frames available for allocation again.
pub fn init(memory_map: &[UEFIMemoryRegion], kernel_start: usize, kernel_end: usize) {
    if MEMORY_INIT_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        log::error!("Memory initialization requested more than once; ignoring duplicate call");
        return;
    }
    log::info!("Initializing memory management subsystem...");
    
    // Calculate total usable memory
    let mut total_usable_bytes = 0usize;
    for region in memory_map {
        if region.ty == UEFIMemoryType::CONVENTIONAL {
            total_usable_bytes = total_usable_bytes.saturating_add(region.size);
        }
    }
    
    log::info!("Total usable RAM: {} MB", total_usable_bytes / (1024 * 1024));
    
    // Initialize the frame allocator
    let mut allocator = FRAME_ALLOCATOR.lock();
    if allocator.is_some() {
        log::error!("Memory subsystem is already initialized; refusing to reset frame ownership");
        return;
    }
    *allocator = Some(PhysicalFrameAllocator::new(memory_map, kernel_start, kernel_end));
    
    if let Some(ref alloc) = *allocator {
        log::info!("Frame allocator initialized: {} frames ({:.2} GB)", 
                   alloc.total_frames(),
                   (alloc.total_frames() * 4096) as f64 / (1024.0 * 1024.0 * 1024.0));
    }
    
    // Initialize paging subsystem
    paging::init();
    
    log::info!("Memory management subsystem initialized");
}

/// Get a reference to the global frame allocator
/// Panics if the allocator has not been initialized
pub fn get_frame_allocator() -> spin::MutexGuard<'static, Option<PhysicalFrameAllocator>> {
    FRAME_ALLOCATOR.lock()
}

/// Allocate a physical frame
/// Returns the physical address of the allocated frame, or None if out of memory
pub fn allocate_frame() -> Option<x86_64::PhysAddr> {
    let mut allocator = get_frame_allocator();
    if let Some(ref mut alloc) = *allocator {
        alloc.allocate().map(|addr| x86_64::PhysAddr::new(addr as u64))
    } else {
        None
    }
}

/// Deallocate a physical frame
///
/// # Safety
///
/// The caller must ensure:
/// - The frame was previously allocated
/// - The frame is not currently in use
/// - No references to this frame exist
pub unsafe fn deallocate_frame(addr: x86_64::PhysAddr) {
    let mut allocator = get_frame_allocator();
    if let Some(ref mut alloc) = *allocator {
        // SAFETY: this wrapper inherits the caller's contract that the frame was
        // allocated by KeefeOS and is no longer referenced or mapped. The
        // allocator additionally rejects frames outside usable physical memory.
        unsafe { alloc.deallocate(addr.as_u64() as usize) };
    }
}

/// Get memory statistics
pub fn get_memory_stats() -> MemoryStats {
    let allocator = get_frame_allocator();
    if let Some(ref alloc) = *allocator {
        MemoryStats {
            addressable_frames: alloc.total_frames(),
            occupied_frames: alloc.allocated_count(),
            free_frames: alloc.free_count(),
            addressable_bytes: alloc.total_frames() * FRAME_SIZE,
            occupied_bytes: alloc.allocated_count() * FRAME_SIZE,
            free_bytes: alloc.free_count() * FRAME_SIZE,
        }
    } else {
        MemoryStats::default()
    }
}

/// Snapshot of the physical-frame allocator's counters.
///
/// These metrics distinguish the tracked physical address span from usable
/// memory. The addressable span includes gaps and reserved firmware ranges;
/// occupied frames include reserved/unavailable frames and are not equivalent
/// to live allocations. Free frames are only complete pages backed by usable
/// conventional memory descriptors.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Number of frame slots between physical address zero and the highest tracked range.
    pub addressable_frames: usize,
    /// Number of frame slots that are reserved, unavailable, or currently allocated.
    pub occupied_frames: usize,
    /// Number of currently available full conventional-memory frames.
    pub free_frames: usize,
    /// Address span represented by the allocator; not a measure of installed RAM.
    pub addressable_bytes: usize,
    /// Occupied/reserved/gap span; not a measure of live memory use.
    pub occupied_bytes: usize,
    /// Currently available conventional RAM represented by free frames.
    pub free_bytes: usize,
}

impl MemoryStats {
    /// Return the percentage of the tracked address span that is occupied or reserved.
    ///
    /// This is not an OS workload memory-usage percentage because reserved regions
    /// and physical address holes count as occupied.
    pub fn occupied_span_percent(&self) -> f64 {
        if self.addressable_frames == 0 {
            0.0
        } else {
            (self.occupied_frames as f64 / self.addressable_frames as f64) * 100.0
        }
    }
}

/// UEFI memory types retain the specification's uppercase identifiers.
/// Based on the UEFI specification.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UEFIMemoryType {
    RESERVED = 0,
    LOADER_CODE = 1,
    LOADER_DATA = 2,
    BOOT_SERVICES_CODE = 3,
    BOOT_SERVICES_DATA = 4,
    RUNTIME_SERVICES_CODE = 5,
    RUNTIME_SERVICES_DATA = 6,
    CONVENTIONAL = 7,
    UNUSABLE_MEMORY = 8,
    ACPI_RECLAIMABLE = 9,
    ACPI_NON_VOLATILE = 10,
    MEMORY_MAPPED_IO = 11,
    MEMORY_MAPPED_IO_PORT_SPACE = 12,
    PAL_CODE = 13,
    PERSISTENT_MEMORY = 14,
}

impl From<u32> for UEFIMemoryType {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::RESERVED,
            1 => Self::LOADER_CODE,
            2 => Self::LOADER_DATA,
            3 => Self::BOOT_SERVICES_CODE,
            4 => Self::BOOT_SERVICES_DATA,
            5 => Self::RUNTIME_SERVICES_CODE,
            6 => Self::RUNTIME_SERVICES_DATA,
            7 => Self::CONVENTIONAL,
            8 => Self::UNUSABLE_MEMORY,
            9 => Self::ACPI_RECLAIMABLE,
            10 => Self::ACPI_NON_VOLATILE,
            11 => Self::MEMORY_MAPPED_IO,
            12 => Self::MEMORY_MAPPED_IO_PORT_SPACE,
            13 => Self::PAL_CODE,
            14 => Self::PERSISTENT_MEMORY,
            _ => Self::RESERVED,
        }
    }
}

/// UEFI Memory Region descriptor
#[derive(Debug, Clone, Copy)]
pub struct UEFIMemoryRegion {
    pub ty: UEFIMemoryType,
    pub phys_start: usize,
    pub size: usize,
}

impl UEFIMemoryRegion {
    /// Check if this region contains usable RAM
    pub fn is_usable(&self) -> bool {
        self.ty == UEFIMemoryType::CONVENTIONAL
    }
    
    /// Return the exclusive end address of this region, or `None` on overflow.
    ///
    /// This computes metadata only; callers must still validate that the region
    /// came from a trusted, structurally validated firmware memory map.
    pub fn end(&self) -> Option<usize> {
        self.phys_start.checked_add(self.size)
    }
}

#[cfg(test)]
mod memory_region_tests {
    use super::{UEFIMemoryRegion, UEFIMemoryType};

    #[test]
    fn checked_end_returns_the_exclusive_address() {
        let region = UEFIMemoryRegion {
            ty: UEFIMemoryType::CONVENTIONAL,
            phys_start: 0x1000,
            size: 0x2000,
        };
        assert_eq!(region.end(), Some(0x3000));
    }

    #[test]
    fn checked_end_rejects_overflow() {
        let region = UEFIMemoryRegion {
            ty: UEFIMemoryType::CONVENTIONAL,
            phys_start: usize::MAX - 0xfff,
            size: 0x2000,
        };
        assert_eq!(region.end(), None);
    }

    #[test]
    fn unknown_uefi_memory_types_are_reserved_by_default() {
        assert_eq!(UEFIMemoryType::from(u32::MAX), UEFIMemoryType::RESERVED);
    }
}
