//! Minimal early-boot bump allocator.
//!
//! This allocator is for bootstrapping only. It cannot reclaim freed objects,
//! so it must not be treated as a general-purpose long-running heap allocator.
//! The fixed heap lives in kernel BSS and requires the identity-mapped load
//! layout that the current linker script requests.

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 8 * 1024 * 1024;

#[repr(align(4096))]
struct Heap([u8; HEAP_SIZE]);

static mut HEAP: Heap = Heap([0; HEAP_SIZE]);

struct EarlyBumpAllocator {
    offset: AtomicUsize,
}

impl EarlyBumpAllocator {
    const fn new() -> Self {
        Self {
            offset: AtomicUsize::new(0),
        }
    }
}

// SAFETY: the allocator is shared through a global static. Atomic compare_exchange
// reserves disjoint byte ranges, the backing heap is static and aligned, and all
// allocator state shared across threads/CPUs is contained in the atomic offset.
unsafe impl Sync for EarlyBumpAllocator {}

// SAFETY: allocations are disjoint ranges reserved via atomic compare_exchange.
// The backing heap is static, aligned, and remains live for the kernel lifetime.
// dealloc intentionally does nothing; therefore the heap only grows.
unsafe impl GlobalAlloc for EarlyBumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = ptr::addr_of_mut!(HEAP) as *mut u8 as usize;
        let alignment = layout.align();
        let size = layout.size().max(1);

        let mut current = self.offset.load(Ordering::Relaxed);
        loop {
            let Some(current_address) = base.checked_add(current) else {
                return ptr::null_mut();
            };
            let Some(aligned_address) = current_address.checked_add(alignment - 1) else {
                return ptr::null_mut();
            };
            let aligned_address = aligned_address & !(alignment - 1);
            let Some(end_address) = aligned_address.checked_add(size) else {
                return ptr::null_mut();
            };
            let Some(new_offset) = end_address.checked_sub(base) else {
                return ptr::null_mut();
            };
            if new_offset > HEAP_SIZE {
                return ptr::null_mut();
            }

            match self.offset.compare_exchange_weak(
                current,
                new_offset,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return aligned_address as *mut u8,
                Err(observed) => current = observed,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Intentionally no-op. Replace this allocator before adding workload
        // churn, process lifecycle, or other long-running heap users.
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: EarlyBumpAllocator = EarlyBumpAllocator::new();

/// Return heap usage for diagnostics.
pub fn used_bytes() -> usize {
    GLOBAL_ALLOCATOR.offset.load(Ordering::Relaxed)
}

/// Capacity of the early bump heap.
pub const fn capacity_bytes() -> usize {
    HEAP_SIZE
}
