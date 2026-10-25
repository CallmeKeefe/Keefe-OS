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

use core::mem;
use super::UEFIMemoryRegion;

/// Size of a physical page frame in bytes
pub const FRAME_SIZE: usize = 4096;

/// Maximum number of frames supported (adjustable based on target systems)
/// 1M frames = 4GB RAM, 16M frames = 64GB RAM, 256M frames = 1TB RAM
const MAX_FRAMES: usize = 16 * 1024 * 1024; // Support up to 64GB RAM

/// Number of u64 words needed for the bitmap
const BITMAP_WORDS: usize = (MAX_FRAMES + 63) / 64;

/// Physical Frame Allocator using bitmap
pub struct PhysicalFrameAllocator {
    /// Bitmap tracking allocated/free frames
    /// Each bit represents one frame: 0=free, 1=allocated
    bitmap: [u64; BITMAP_WORDS],
    
    /// Total number of frames managed
    total_frames: usize,
    
    /// Number of currently allocated frames
    allocated_count: usize,
    
    /// Starting frame index (first usable frame after kernel)
    start_frame: usize,
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
        // Initialize bitmap to all zeros (all frames free initially)
        let mut bitmap = [0u64; BITMAP_WORDS];
        
        // Find the highest physical address to determine total frames needed
        let mut max_addr = 0usize;
        for region in memory_map {
            let region_end = region.end();
            if region_end > max_addr {
                max_addr = region_end;
            }
        }
        
        // Calculate total frames (capped at MAX_FRAMES)
        let total_frames = core::cmp::min(max_addr / FRAME_SIZE, MAX_FRAMES);
        
        log::info!("PhysicalFrameAllocator: managing {} frames ({:.2} GB)",
                   total_frames,
                   (total_frames * FRAME_SIZE) as f64 / (1024.0 * 1024.0 * 1024.0));
        
        // Mark all non-usable regions as allocated
        for region in memory_map {
            if !region.is_usable() {
                Self::mark_region_allocated(&mut bitmap, region.phys_start, region.size);
            }
        }
        
        // Mark kernel memory as allocated
        let kernel_frames_start = kernel_start / FRAME_SIZE;
        let kernel_frames_end = kernel_end / FRAME_SIZE;
        for frame_idx in kernel_frames_start..=kernel_frames_end {
            if frame_idx < MAX_FRAMES {
                Self::set_bit(&mut bitmap, frame_idx, true);
            }
        }
        
        // Count allocated frames
        let allocated_count = Self::count_allocated(&bitmap, total_frames);
        
        log::info!("Kernel occupies frames {}-{} ({} MB)",
                   kernel_frames_start,
                   kernel_frames_end,
                   (kernel_frames_end - kernel_frames_start) * FRAME_SIZE / (1024 * 1024));
        
        Self {
            bitmap,
            total_frames,
            allocated_count,
            start_frame: kernel_frames_end + 1,
        }
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
        for (word_idx, word) in self.bitmap.iter_mut().enumerate() {
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
    
    /// Deallocate a physical frame
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The frame was previously allocated
    /// - The frame is not currently in use
    /// - No references to this frame exist
    pub fn deallocate(&mut self, phys_addr: usize) {
        let frame_idx = phys_addr / FRAME_SIZE;
        
        if frame_idx >= self.total_frames {
            log::error!("Attempted to deallocate invalid frame {} (max: {})",
                        frame_idx, self.total_frames);
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
    
    /// Mark a memory region as allocated in the bitmap
    fn mark_region_allocated(bitmap: &mut [u64; BITMAP_WORDS], start: usize, size: usize) {
        let start_frame = start / FRAME_SIZE;
        let end_frame = (start + size + FRAME_SIZE - 1) / FRAME_SIZE;
        
        for frame_idx in start_frame..end_frame {
            if frame_idx < MAX_FRAMES {
                Self::set_bit(bitmap, frame_idx, true);
            }
        }
    }
    
    /// Count the number of allocated frames in the bitmap
    fn count_allocated(bitmap: &[u64; BITMAP_WORDS], total_frames: usize) -> usize {
        let mut count = 0;
        let words_needed = total_frames.saturating_add(63) / 64;
        for (word_idx, word) in bitmap.iter().take(words_needed).enumerate() {
            let valid_frames = total_frames.saturating_sub(word_idx * 64).min(64);
            // Ignore bitmap padding beyond the highest usable physical frame.
            count += (word.count_ones() as usize).min(valid_frames);
        }
        count
    }
    
    /// Get the value of a specific bit in the bitmap
    #[inline]
    fn get_bit(bitmap: &[u64; BITMAP_WORDS], bit_index: usize) -> bool {
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;
        
        if word_idx >= BITMAP_WORDS {
            return false;
        }
        
        (bitmap[word_idx] & (1 << bit_offset)) != 0
    }
    
    /// Set a specific bit in the bitmap
    #[inline]
    fn set_bit(bitmap: &mut [u64; BITMAP_WORDS], bit_index: usize, value: bool) {
        let word_idx = bit_index / 64;
        let bit_offset = bit_index % 64;
        
        if word_idx >= BITMAP_WORDS {
            return;
        }
        
        if value {
            bitmap[word_idx] |= 1 << bit_offset;
        } else {
            bitmap[word_idx] &= !(1 << bit_offset);
        }
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
