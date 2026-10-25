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
//! 1. **O(1) Allocation/Deallocation**: Finding and toggling a bit is constant time
//! 2. **Cache Efficiency**: The bitmap is compact and fits in cache
//! 3. **Fragmentation Tracking**: Easy to detect and handle fragmentation
//! 4. **NUMA Ready**: Can be extended with per-NUMA-node bitmaps
//! 5. **Security**: Clear visualization of all allocated frames
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

use self::frame_allocator::PhysicalFrameAllocator;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Global physical frame allocator
/// This is initialized once during boot and used throughout the kernel lifetime
static FRAME_ALLOCATOR: Mutex<Option<PhysicalFrameAllocator>> = Mutex::new(None);

/// Total number of physical frames managed by the allocator
static TOTAL_FRAMES: AtomicUsize = AtomicUsize::new(0);

/// Number of currently allocated frames
static ALLOCATED_FRAMES: AtomicUsize = AtomicUsize::new(0);

/// Initialize the memory management subsystem
///
/// # Arguments
///
/// * `memory_map` - UEFI memory map indicating usable and reserved regions
/// * `kernel_start` - Physical start address of the kernel
/// * `kernel_end` - Physical end address of the kernel
///
/// # Safety
///
/// This function must be called exactly once during system initialization.
/// Calling it multiple times will cause memory corruption.
pub fn init(memory_map: &[UEFIMemoryRegion], kernel_start: usize, kernel_end: usize) {
    log::info!("Initializing memory management subsystem...");
    
    // Calculate total usable memory
    let mut total_usable_bytes = 0usize;
    for region in memory_map {
        if region.ty == UEFIMemoryType::CONVENTIONAL {
            total_usable_bytes += region.size;
        }
    }
    
    log::info!("Total usable RAM: {} MB", total_usable_bytes / (1024 * 1024));
    
    // Initialize the frame allocator
    let mut allocator = FRAME_ALLOCATOR.lock();
    *allocator = Some(PhysicalFrameAllocator::new(memory_map, kernel_start, kernel_end));
    
    if let Some(ref alloc) = *allocator {
        TOTAL_FRAMES.store(alloc.total_frames(), Ordering::Relaxed);
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
        alloc.deallocate(addr.as_u64() as usize);
    }
}

/// Get memory statistics
pub fn get_memory_stats() -> MemoryStats {
    let allocator = get_frame_allocator();
    if let Some(ref alloc) = *allocator {
        MemoryStats {
            total_frames: alloc.total_frames(),
            allocated_frames: alloc.allocated_count(),
            free_frames: alloc.free_count(),
            total_bytes: alloc.total_frames() * 4096,
            allocated_bytes: alloc.allocated_count() * 4096,
            free_bytes: alloc.free_count() * 4096,
        }
    } else {
        MemoryStats::default()
    }
}

/// Memory statistics structure
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    pub total_frames: usize,
    pub allocated_frames: usize,
    pub free_frames: usize,
    pub total_bytes: usize,
    pub allocated_bytes: usize,
    pub free_bytes: usize,
}

impl MemoryStats {
    /// Get memory usage as a percentage
    pub fn usage_percent(&self) -> f64 {
        if self.total_frames == 0 {
            0.0
        } else {
            (self.allocated_frames as f64 / self.total_frames as f64) * 100.0
        }
    }
}

/// UEFI Memory Region types
/// Based on the UEFI specification
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
    
    /// Get the end address of this region
    pub fn end(&self) -> usize {
        self.phys_start + self.size
    }
}
