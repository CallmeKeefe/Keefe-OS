//! Physical Frame Allocator (Bitmap-Based)
//!
//! This module implements a bitmap-based physical frame allocator.
//! Each bit represents one 4KB physical page frame.
//!
//! # Data Structure
//!
//! ```text
//! Bitmap Layout:
//! +-------+-------+-------+-------+
//! | u64   | u64   | u64   | u64   | ...
//! +-------+-------+-------+-------+
//! | 64    | 64    | 64    | 64    | bits per word
//! +-------+-------+-------+-------+
//! 
//! Bit = 0: Frame is FREE
//! Bit = 1: Frame is ALLOCATED
//! ```
//!
//! # Performance Characteristics
//!
//! - **Allocation**: O(n/64) where n is the number of frames
//!   - In practice, very fast due to bit scanning instructions
//! - **Deallocation**: O(1) - just clear a bit
//! - **Memory Overhead**: 1 bit per 4KB = 0.003% overhead
//!   - Example: 64GB RAM needs only ~2MB for the bitmap
//!
//! # Thread Safety
//!
//! This allocator is protected by a spinlock in the parent module.
//! For lock-free operation in multi-core systems, consider:
//! - Using atomic operations on individual bitmap words
//! - Implementing per-CPU caches with work stealing

use alloc::vec;
use super::UEFIMemoryRegion;

/// Size of a physical page frame in bytes
pub const FRAME_SIZE: usize = 4096;

/// Maximum number of frames supported (adjustable based on target systems)
/// 1M frames = 4GB RAM, 16M frames = 64GB RAM, 256M frames = 1TB RAM
const MAX_FRAMES: usize = 16 * 1024 * 1024; // Support up to 64GB RAM

/// Physical Frame Allocator using bitmap
pub struct PhysicalFrameAllocator {
    /// True means the frame is occupied/reserved; false means free.
    bitmap: alloc::vec::Vec<u64>,

    /// True only for pages completely inside usable memory and not reserved
    /// by the firmware map, kernel image, or low-memory policy.
    allocatable: alloc::vec::Vec<u64>,

    /// Total number of frames represented by these bitmaps.
    total_frames: usize,

    /// Number of occupied/reserved frames in the managed address range.
    allocated_count: usize,
}

impl PhysicalFrameAllocator {
    /// Create a new frame allocator from the UEFI memory map
    ///
    /// # Arguments
    ///
    /// * `memory_map` - UEFI-provided memory regions
    /// * `kernel_start` - Physical address where the kernel begins
    /// * `kernel_end` - Physical address where the kernel ends
    ///
    /// # Returns
    ///
    /// A new `PhysicalFrameAllocator` instance
    pub fn new(
        memory_map: &[UEFIMemoryRegion],
        kernel_start: usize,
        kernel_end: usize,
    ) -> Self {
        let mut max_addr = 0usize;
        for region in memory_map {
            if let Some(region_end) = region.phys_start.checked_add(region.size) {
                max_addr = max_addr.max(region_end);
            } else {
                log::error!("Ignoring overflowing physical memory range during allocator sizing");
            }
        }

        let rounded_max = max_addr.saturating_add(FRAME_SIZE - 1);
        let total_frames = core::cmp::min(rounded_max / FRAME_SIZE, MAX_FRAMES);
        let bitmap_words = total_frames.saturating_add(63) / 64;

        // Start with every represented frame occupied and none eligible for
        // release. Size bitmaps to the detected memory map, not the 64 GiB cap.
        let mut bitmap = vec![u64::MAX; bitmap_words];
        let mut allocatable = vec![0u64; bitmap_words];

        log::info!(
            "PhysicalFrameAllocator: managing {} frames ({:.2} GiB addressable)",
            total_frames,
            (total_frames * FRAME_SIZE) as f64 / (1024.0 * 1024.0 * 1024.0)
        );

        // First make only explicitly usable conventional memory available.
        for region in memory_map {
            if region.is_usable() {
                Self::mark_region_free(
                    &mut bitmap, &mut allocatable, region.phys_start, region.size, total_frames,
                );
            }
        }

        // Reserved/non-usable descriptors always win if malformed or overlapping
        // descriptors are supplied. Never let a usable range expose a page that
        // any firmware descriptor says is reserved or device memory.
        for region in memory_map {
            if !region.is_usable() {
                Self::mark_region_allocated(
                    &mut bitmap, &mut allocatable, region.phys_start, region.size, total_frames,
                );
            }
        }

        // Keep legacy low memory reserved even if firmware labels a part usable.
        Self::mark_region_allocated(&mut bitmap, &mut allocatable, 0, 0x10_0000, total_frames);
        // Treat kernel_end as exclusive when reserving kernel pages.
        if kernel_end > kernel_start {
            Self::mark_region_allocated(
                &mut bitmap,
                &mut allocatable,
                kernel_start,
                kernel_end - kernel_start,
                total_frames,
            );
        }

        let allocated_count = Self::count_allocated(&bitmap, total_frames);
        Self { bitmap, allocatable, total_frames, allocated_count }
    }

    /// Allocate a single physical frame
    ///
    /// Returns the physical address of the allocated frame,
    /// or None if no frames are available.
    ///
    /// # Algorithm
    ///
    /// Uses efficient bit scanning to find the first zero bit:
    /// 1. Scan bitmap words looking for non-zero (has free bit)
    /// 2. Use trailing_zeros() to find the exact bit position
    /// 3. Set the bit and return the frame address
    pub fn allocate(&mut self) -> Option<usize> {
        // Search for a free frame (bit = 0)
        let words_to_scan = self.total_frames.saturating_add(63) / 64;
        for (word_idx, word) in self.bitmap.iter_mut().take(words_to_scan).enumerate() {
            if *word != u64::MAX {
                // This word has at least one free bit
                let free_bit = word.trailing_ones();
                if free_bit < 64 {
                    let frame_idx = word_idx * 64 + free_bit as usize;
                    
                    if frame_idx < self.total_frames {
                        // Mark as allocated
                        *word |= 1 << free_bit;
                        self.allocated_count += 1;
                        
                        let phys_addr = frame_idx * FRAME_SIZE;
                        log::trace!("Allocated frame {} at {:#x}", frame_idx, phys_addr);
                        
                        return Some(phys_addr);
                    }
                }
            }
        }
        
        // No free frames available
        log::warn!("Out of physical memory! Allocated: {}/{}",
                   self.allocated_count, self.total_frames);
        None
    }
    
    /// Allocate multiple contiguous physical frames
    ///
    /// # Arguments
    ///
    /// * `count` - Number of contiguous frames to allocate
    ///
    /// # Returns
    ///
    /// The physical address of the first frame, or None if not enough
    /// contiguous frames are available.
    ///
    /// # Note
    ///
    /// This is a simple implementation that may be slow for large allocations.
    /// For production use, consider implementing a buddy allocator or
    /// using a more sophisticated algorithm.
    pub fn allocate_contiguous(&mut self, count: usize) -> Option<usize> {
        if count == 0 || count > self.total_frames {
            return None;
        }
        
        // Simple linear search for contiguous free frames
        let mut consecutive_free = 0;
        let mut start_idx = None;
        
        for frame_idx in 0..self.total_frames {
            if !Self::get_bit(&self.bitmap, frame_idx) {
                // Frame is free
                if consecutive_free == 0 {
                    start_idx = Some(frame_idx);
                }
                consecutive_free += 1;
                
                if consecutive_free >= count {
                    // Found enough contiguous frames
                    break;
                }
            } else {
                // Frame is allocated, reset counter
                consecutive_free = 0;
                start_idx = None;
            }
        }
        
        if let Some(start) = start_idx {
            if consecutive_free >= count {
                // Mark all frames as allocated
                for i in 0..count {
                    Self::set_bit(&mut self.bitmap, start + i, true);
                }
                self.allocated_count += count;
                
                return Some(start * FRAME_SIZE);
            }
        }
        
        None
    }
    
    /// Release a dynamically allocated physical frame.
    ///
    /// Frames that are reserved, firmware-owned, outside usable conventional
    /// memory, or outside the managed range are rejected defensively.
    ///
    /// # Safety
    ///
    /// The caller must ensure the frame was returned by this allocator, is no
    /// longer in use, and has no outstanding references or mappings. The
    /// allocator can enforce pool membership, but it cannot prove those
    /// ownership/lifetime conditions.
    pub unsafe fn deallocate(&mut self, phys_addr: usize) {
        if phys_addr % FRAME_SIZE != 0 {
            log::error!("Attempted to deallocate unaligned physical address {:#x}", phys_addr);
            return;
        }
        let frame_idx = phys_addr / FRAME_SIZE;

        if frame_idx >= self.total_frames {
            log::error!("Attempted to deallocate invalid frame {} (max: {})",
                        frame_idx, self.total_frames);
            return;
        }
        
        if !Self::get_bit(&self.allocatable, frame_idx) {
            log::error!(
                "Refusing to deallocate frame {:#x} outside the usable allocation pool",
                phys_addr
            );
            return;
        }

        if !Self::get_bit(&self.bitmap, frame_idx) {
            log::warn!("Double-free detected for frame {}", frame_idx);
            return;
        }
        
        Self::set_bit(&mut self.bitmap, frame_idx, false);
        self.allocated_count -= 1;
        
        log::trace!("Deallocated frame {} at {:#x}", frame_idx, phys_addr);
    }
    
    /// Get the total number of frames managed
    #[inline]
    pub const fn total_frames(&self) -> usize {
        self.total_frames
    }
    
    /// Get the number of allocated frames
    #[inline]
    pub const fn allocated_count(&self) -> usize {
        self.allocated_count
    }
    
    /// Get the number of free frames
    #[inline]
    pub fn free_count(&self) -> usize {
        self.total_frames - self.allocated_count
    }
    
    /// Check if a specific frame is allocated
    #[inline]
    pub fn is_allocated(&self, frame_idx: usize) -> bool {
        if frame_idx >= self.total_frames {
            return false;
        }
        Self::get_bit(&self.bitmap, frame_idx)
    }
    
    /// Mark all frames intersecting a memory range as reserved.
    /// The bitmap defaults to reserved, so invalid ranges remain unavailable.
    fn mark_region_allocated(
        bitmap: &mut [u64],
        allocatable: &mut [u64],
        start: usize,
        size: usize,
        total_frames: usize,
    ) {
        if size == 0 {
            return;
        }
        let Some(end_addr) = start.checked_add(size) else {
            return;
        };
        let end_frame = end_addr.saturating_add(FRAME_SIZE - 1) / FRAME_SIZE;
        for frame_idx in (start / FRAME_SIZE).min(total_frames)..end_frame.min(total_frames) {
            Self::set_bit(bitmap, frame_idx, true);
            Self::set_bit(allocatable, frame_idx, false);
        }
    }

    /// Make only explicit conventional-memory frames available.
    fn mark_region_free(
        bitmap: &mut [u64],
        allocatable: &mut [u64],
        start: usize,
        size: usize,
        total_frames: usize,
    ) {
        if size == 0 {
            return;
        }
        let Some(end_addr) = start.checked_add(size) else {
            log::error!("Ignoring overflowing usable memory range");
            return;
        };
        // Free only frames wholly contained within the declared usable range.
        // Rounding the start down or end up here could expose a partially
        // covered frame if a malformed/unexpected descriptor reaches this helper.
        let Some(start_rounded) = start.checked_add(FRAME_SIZE - 1) else {
            log::error!("Ignoring unrepresentable usable memory start");
            return;
        };
        let start_frame = start_rounded / FRAME_SIZE;
        let end_frame = end_addr / FRAME_SIZE; // exclusive; round end down
        for frame_idx in start_frame.min(total_frames)..end_frame.min(total_frames) {
            Self::set_bit(bitmap, frame_idx, false);
            Self::set_bit(allocatable, frame_idx, true);
        }
    }

    /// Count the number of allocated frames in the bitmap
    fn count_allocated(bitmap: &[u64], total_frames: usize) -> usize {
        let mut count = 0;
        let words_needed = total_frames.saturating_add(63) / 64;
        for (word_idx, word) in bitmap.iter().take(words_needed).enumerate() {
            let valid_frames = total_frames.saturating_sub(word_idx * 64).min(64);
            // Mask padding bits before counting: the bitmap starts with all bits
            // reserved, including bits beyond total_frames in the final word.
            let valid_mask = match valid_frames {
                0 => 0,
                64 => u64::MAX,
                n => (1u64 << n) - 1,
            };
            count += (word & valid_mask).count_ones() as usize;
        }
        count
    }
    
    /// Get the value of a specific bit in the bitmap
    #[inline]
    fn get_bit(bitmap: &[u64], bit_index: usize) -> bool {
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;
        
        if word_idx >= bitmap.len() {
            return false;
        }
        
        (bitmap[word_idx] & (1 << bit_offset)) != 0
    }
    
    /// Set a specific bit in the bitmap
    #[inline]
    fn set_bit(bitmap: &mut [u64], bit_index: usize, value: bool) {
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;
        
        if word_idx >= bitmap.len() {
            return;
        }
        
        if value {
            bitmap[word_idx] |= 1 << bit_offset;
        } else {
            bitmap[word_idx] &= !(1 << bit_offset);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::UEFIMemoryType;

    fn region(ty: UEFIMemoryType, start: usize, size: usize) -> UEFIMemoryRegion {
        UEFIMemoryRegion { ty, phys_start: start, size }
    }

    #[test]
    fn only_conventional_regions_are_allocatable() {
        let map = [
            region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x4000),
            region(UEFIMemoryType::RESERVED, 0x20_0000, 0x4000),
            region(UEFIMemoryType::CONVENTIONAL, 0x20_4000, 0x4000),
        ];
        let alloc = PhysicalFrameAllocator::new(&map, 0x10_1000, 0x10_3000);
        assert!(alloc.is_allocated(0)); // low memory
        assert!(alloc.is_allocated(0x10_1000 / FRAME_SIZE)); // kernel
        assert!(alloc.is_allocated(0x104000 / FRAME_SIZE)); // gap
        assert!(alloc.is_allocated(0x200000 / FRAME_SIZE)); // reserved descriptor
        assert!(!alloc.is_allocated(0x103000 / FRAME_SIZE)); // conventional memory
        assert!(!alloc.is_allocated(0x204000 / FRAME_SIZE)); // later conventional range
    }

    #[test]
    fn unaligned_usable_ranges_only_free_wholly_contained_frames() {
        // The first page is missing its first byte and the last page is missing
        // its final byte because the descriptor end is exclusive.
        let map = [region(UEFIMemoryType::CONVENTIONAL, 0x10_0001, 0x2ffe)];
        let alloc = PhysicalFrameAllocator::new(&map, 0x40_0000, 0x40_1000);

        assert!(alloc.is_allocated(0x10_0000 / FRAME_SIZE));
        assert!(!alloc.is_allocated(0x10_1000 / FRAME_SIZE));
        assert!(alloc.is_allocated(0x10_2000 / FRAME_SIZE));
        assert!(alloc.is_allocated(0x10_3000 / FRAME_SIZE));
    }

    #[test]
    fn reserved_descriptor_wins_when_memory_ranges_overlap() {
        let map = [
            region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x4000),
            region(UEFIMemoryType::RESERVED, 0x10_1000, 0x1000),
        ];
        let alloc = PhysicalFrameAllocator::new(&map, 0x40_0000, 0x40_1000);

        assert!(!alloc.is_allocated(0x10_0000 / FRAME_SIZE));
        assert!(alloc.is_allocated(0x10_1000 / FRAME_SIZE));
        assert!(!alloc.is_allocated(0x10_2000 / FRAME_SIZE));
    }

    #[test]
    fn reserved_frames_cannot_be_returned_to_the_free_pool() {
        let map = [
            region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x4000),
            region(UEFIMemoryType::RESERVED, 0x20_0000, 0x1000),
        ];
        let mut alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        let before = alloc.free_count();
        let reserved_frame = 0x20_0000 / FRAME_SIZE;

        assert!(alloc.is_allocated(reserved_frame));
        unsafe { alloc.deallocate(0x20_0000) };

        assert!(alloc.is_allocated(reserved_frame));
        assert_eq!(alloc.free_count(), before);
        assert_ne!(alloc.allocate(), Some(0x20_0000));
    }

    #[test]
    fn bitmap_is_sized_to_the_addressable_memory_map() {
        let map = [region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x4000)];
        let alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        assert!(alloc.bitmap.len() < 1024);
        assert_eq!(alloc.bitmap.len(), alloc.allocatable.len());
    }

    #[test]
    fn allocation_skips_physical_holes() {
        let map = [
            region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x2000),
            region(UEFIMemoryType::CONVENTIONAL, 0x20_0000, 0x2000),
        ];
        let mut alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        assert_eq!(alloc.allocate(), Some(0x10_1000));
        assert_eq!(alloc.allocate(), Some(0x20_0000));
    }

    #[test]
    fn deallocation_requires_page_alignment() {
        let map = [region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x3000)];
        let mut alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        let addr = alloc.allocate().expect("usable frame should be available");
        let before = alloc.free_count();
        unsafe { alloc.deallocate(addr + 1) };
        assert_eq!(alloc.free_count(), before);
        unsafe { alloc.deallocate(addr) };
        assert_eq!(alloc.free_count(), before + 1);
    }

    #[test]
    fn padding_bits_do_not_count_as_managed_frames() {
        let map = [region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x3000)];
        let alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        assert_eq!(alloc.total_frames(), (0x103000usize + FRAME_SIZE - 1) / FRAME_SIZE);
        assert_eq!(alloc.allocated_count() + alloc.free_count(), alloc.total_frames());
        assert_eq!(alloc.free_count(), 2);
    }

    #[test]
    fn kernel_end_is_exclusive() {
        let map = [region(UEFIMemoryType::CONVENTIONAL, 0x10_0000, 0x4000)];
        let alloc = PhysicalFrameAllocator::new(&map, 0x10_0000, 0x10_1000);
        assert!(alloc.is_allocated(0x10_0000 / FRAME_SIZE));
        assert!(!alloc.is_allocated(0x10_1000 / FRAME_SIZE));
    }
}

/// Implementation of x86_64's FrameAllocator trait for interoperability
unsafe impl x86_64::structures::paging::FrameAllocator<x86_64::structures::paging::Size4KiB>
    for PhysicalFrameAllocator
{
    fn allocate_frame(
        &mut self,
    ) -> Option<x86_64::structures::paging::PhysFrame<x86_64::structures::paging::Size4KiB>>
    {
        self.allocate().and_then(|addr| {
            let physical = x86_64::PhysAddr::new(addr as u64);
            x86_64::structures::paging::PhysFrame::from_start_address(physical).ok()
        })
    }
}
