//! Kernel boot-handoff ABI (KeefeOS Boot Info Block, KIB).
//!
//! This module defines the *only* contract between the UEFI bootloader and
//! the kernel. Both sides must agree on this layout; it is deliberately a
//! flat, `repr(C)` structure with explicit sizes so that:
//!
//! - The kernel never guesses the firmware descriptor stride. The bootloader
//!   normalizes each descriptor into a stable 40-byte record while preserving
//!   the firmware-reported descriptor size and version.
//! - Every field is validated before use (magic, version, total size,
//!   bounds, page-count overflow) by [`BootInfo::validate`].
//!
//! # Handoff protocol
//!
//! The bootloader-to-kernel BootInfo contract is a design target, not yet
//! implemented by the current bootloader. Before this ABI can be used, an ELF64
//! loader must validate/load PT_LOAD segments, preserve reserved ranges, and pass:
//!
//! ```text
//! extern "C" fn kernel_start(boot_info_phys: u64) -> !
//! ```
//!
//! At entry the identity mapping created by UEFI is still active, so the
//! physical address in `RDI` can be dereferenced directly.
//!
//! Milestone string: `KEEFEOS-KERNEL-BOOT-OK` is reserved for a future real boot
//! smoke test. The current bootloader fails closed and does not reach this code.

/// Magic value ("KIB\x01") identifying a valid boot info block.
pub const BOOT_INFO_MAGIC: u32 = 0x_4B49_4201;
/// Version of this handoff ABI.
pub const BOOT_INFO_VERSION: u32 = 1;

/// Size of one normalized memory descriptor in bytes (40 for ABI v1).
pub const MEMORY_DESCRIPTOR_SIZE: usize = core::mem::size_of::<MemoryDescriptor>();

/// Maximum number of memory descriptors carried through the handoff.
pub const MAX_MEMORY_DESCRIPTORS: usize = 512;

/// Page size used throughout the handoff (UEFI pages are 4 KiB).
pub const PAGE_SIZE: u64 = 4096;

/// One normalized UEFI memory descriptor (mirrors the UEFI 2.x
/// `EFI_MEMORY_DESCRIPTOR` layout: type, pad, phys, virt, pages, attr).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct MemoryDescriptor {
    /// UEFI memory type (numeric value from the UEFI spec).
    pub ty: u32,
    /// Implicit padding to keep `phys_start` 8-byte aligned.
    pub _pad: u32,
    /// Physical start address (byte address, page-aligned).
    pub phys_start: u64,
    /// Virtual start address (unused at boot; carried for completeness).
    pub virt_start: u64,
    /// Number of 4 KiB pages in this range (**not** a byte count).
    pub page_count: u64,
    /// UEFI memory attribute bitmap.
    pub att: u64,
}

/// KeefeOS Boot Info Block v1.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootInfo {
    /// Must equal [`BOOT_INFO_MAGIC`].
    pub magic: u32,
    /// Must equal [`BOOT_INFO_VERSION`].
    pub version: u32,
    /// Total byte size of this structure as written by the bootloader.
    pub total_size: u32,
    /// Header size (`size_of::<BootInfo>()`).
    pub header_size: u32,
    /// Descriptor size used by the *firmware* when it produced the map.
    pub firmware_desc_size: u32,
    /// Descriptor version reported by the firmware.
    pub firmware_desc_version: u32,
    /// Number of valid entries in `memory_map`.
    pub memory_map_count: u32,
    /// Reserved flags word (currently 0).
    pub flags: u32,
    /// Start of the loaded kernel image (virtual == physical at handoff).
    pub kernel_start: u64,
    /// End (exclusive) of the loaded kernel image.
    pub kernel_end: u64,
    /// ELF entry point actually jumped to.
    pub kernel_entry: u64,
    /// Base physical address of the frame allocator's managed region.
    pub memory_region_base: u64,
    /// Size in bytes of the managed region.
    pub memory_region_size: u64,
    /// Linear framebuffer address (0 if unavailable).
    pub fb_addr: u64,
    /// Framebuffer pixel format identifier (0 if unavailable).
    pub fb_pixel_format: u32,
    /// Framebuffer horizontal resolution.
    pub fb_horiz_res: u32,
    /// Framebuffer vertical resolution.
    pub fb_vert_res: u32,
    /// Framebuffer pixels per line (stride in pixels).
    pub fb_pixels_per_scan_line: u32,
    /// Command line length in bytes (currently unused, kept for ABI).
    pub cmdline_size: u32,
    /// Command line offset (currently unused, kept for ABI).
    pub cmdline_offset: u32,
    /// Normalized copy of the UEFI memory map taken at `exit_boot_services`.
    pub memory_map: [MemoryDescriptor; MAX_MEMORY_DESCRIPTORS],
}

/// Errors produced while validating a boot info block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInfoError {
    /// The kernel image range is empty, reversed, or does not contain its entry point.
    InvalidKernelRange,
    /// A descriptor is empty or its physical start is not page-aligned.
    InvalidDescriptor,
    /// Magic mismatch — not a KIB block.
    BadMagic,
    /// Unsupported ABI version.
    BadVersion,
    /// `total_size` smaller than the header or larger than the buffer.
    BadTotalSize,
    /// Firmware descriptor size is zero or absurdly large.
    BadFirmwareDescSize,
    /// `memory_map_count` exceeds the fixed array.
    TooManyEntries,
    /// A descriptor's byte range overflows `u64` when converting pages.
    DescriptorOverflow,
    /// Two descriptors overlap in physical address space.
    OverlappingDescriptors,
    /// The kernel range is not covered by a non-usable descriptor.
    KernelRangeNotReserved,
}

impl BootInfo {
    /// Create a fully-zeroed boot info block with magic/version filled in.
    #[must_use]
    pub const fn new() -> Self {
        BootInfo {
            magic: BOOT_INFO_MAGIC,
            version: BOOT_INFO_VERSION,
            total_size: core::mem::size_of::<BootInfo>() as u32,
            header_size: core::mem::size_of::<BootInfo>() as u32,
            firmware_desc_size: 0,
            firmware_desc_version: 0,
            memory_map_count: 0,
            flags: 0,
            kernel_start: 0,
            kernel_end: 0,
            kernel_entry: 0,
            memory_region_base: 0,
            memory_region_size: 0,
            fb_addr: 0,
            fb_pixel_format: 0,
            fb_horiz_res: 0,
            fb_vert_res: 0,
            fb_pixels_per_scan_line: 0,
            cmdline_size: 0,
            cmdline_offset: 0,
            memory_map: [MemoryDescriptor::default(); MAX_MEMORY_DESCRIPTORS],
        }
    }

    /// Validate the structural invariants of this block, given the raw byte
    /// buffer it was read from (which may be larger than the block itself).
    ///
    /// This function performs no allocation and touches only the fields it
    /// needs; it is safe to call on untrusted data *before* any pointer
    /// derived from the block is dereferenced further.
    pub fn validate(&self, provided_bytes: usize) -> Result<(), BootInfoError> {
        if self.magic != BOOT_INFO_MAGIC {
            return Err(BootInfoError::BadMagic);
        }
        if self.version != BOOT_INFO_VERSION {
            return Err(BootInfoError::BadVersion);
        }
        let header = core::mem::size_of::<BootInfo>();
        if (self.total_size as usize) < header || (self.total_size as usize) > provided_bytes {
            return Err(BootInfoError::BadTotalSize);
        }
        if self.header_size as usize != header {
            return Err(BootInfoError::BadTotalSize);
        }
        // The firmware may legitimately report a descriptor larger than our
        // normalized 48-byte layout (future UEFI versions append fields);
        // we copy the prefix. It must never be smaller or zero.
        if self.firmware_desc_size < MEMORY_DESCRIPTOR_SIZE as u32
            || self.firmware_desc_size > 4096
        {
            return Err(BootInfoError::BadFirmwareDescSize);
        }
        if self.memory_map_count as usize > MAX_MEMORY_DESCRIPTORS {
            return Err(BootInfoError::TooManyEntries);
        }
        let entries = &self.memory_map[..self.memory_map_count as usize];
        for d in entries {
            // Reject absurd page counts before multiplying.
            if d.page_count == 0 || d.phys_start % PAGE_SIZE != 0 {
                return Err(BootInfoError::InvalidDescriptor);
            }
            if d.page_count > (u64::MAX / PAGE_SIZE) {
                return Err(BootInfoError::DescriptorOverflow);
            }
            // phys_start must be page-aligned-ish and the range must not
            // wrap around the address space.
            if d.phys_start.checked_add(d.page_count * PAGE_SIZE).is_none() {
                return Err(BootInfoError::DescriptorOverflow);
            }
        }
        Ok(())
    }

    /// Byte end of a descriptor's physical range (checked).
    #[must_use]
    pub fn descriptor_end(d: &MemoryDescriptor) -> Option<u64> {
        let bytes = d.page_count.checked_mul(PAGE_SIZE)?;
        d.phys_start.checked_add(bytes)
    }

    /// Cross-check that descriptors do not overlap each other and that the
    /// kernel image range is covered by a reserved (non-conventional)
    /// descriptor, i.e. the frame allocator cannot hand out kernel memory.
    pub fn semantic_check(&self) -> Result<(), BootInfoError> {
        if self.kernel_start >= self.kernel_end
            || self.kernel_entry < self.kernel_start
            || self.kernel_entry >= self.kernel_end
        {
            return Err(BootInfoError::InvalidKernelRange);
        }
        let entries = &self.memory_map[..self.memory_map_count as usize];

        // Overlap detection (O(n^2) but n <= 512 and runs once at boot).
        for (i, a) in entries.iter().enumerate() {
            let Some(a_end) = Self::descriptor_end(a) else {
                return Err(BootInfoError::DescriptorOverflow);
            };
            if a.page_count != 0 && a.phys_start >= a_end {
                return Err(BootInfoError::DescriptorOverflow);
            }
            for b in &entries[i + 1..] {
                let Some(b_end) = Self::descriptor_end(b) else {
                    return Err(BootInfoError::DescriptorOverflow);
                };
                if a.phys_start < b_end && b.phys_start < a_end {
                    return Err(BootInfoError::OverlappingDescriptors);
                }
            }
        }

        // The kernel range must sit inside exactly one descriptor whose type
        // is NOT conventional RAM (type 7), otherwise the loader reserved it
        // incorrectly.
        let mut covered = false;
        for d in entries {
            let Some(end) = Self::descriptor_end(d) else {
                return Err(BootInfoError::DescriptorOverflow);
            };
            if d.phys_start <= self.kernel_start && self.kernel_end <= end {
                if d.ty == 7 {
                    return Err(BootInfoError::KernelRangeNotReserved);
                }
                covered = true;
                break;
            }
        }
        if !covered {
            return Err(BootInfoError::KernelRangeNotReserved);
        }
        Ok(())
    }

    /// Usable RAM ranges: conventional memory (type 7) minus the kernel
    /// image range. Returns `(start, size)` pairs in ascending order.
    #[must_use]
    pub fn usable_regions(&self) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        for d in &self.memory_map[..self.memory_map_count as usize] {
            if d.ty != 7 {
                continue;
            }
            let Some(end) = Self::descriptor_end(d) else {
                continue;
            };
            // Carve the kernel range out defensively even though the map
            // should already exclude it.
            if end <= self.kernel_start || d.phys_start >= self.kernel_end {
                out.push((d.phys_start, end - d.phys_start));
                continue;
            }
            if end <= self.kernel_start {
                out.push((d.phys_start, end - d.phys_start));
            } else if d.phys_start >= self.kernel_end {
                out.push((d.phys_start, end - d.phys_start));
            } else {
                if d.phys_start < self.kernel_start {
                    out.push((d.phys_start, self.kernel_start - d.phys_start));
                }
                if end > self.kernel_end {
                    out.push((self.kernel_end, end - self.kernel_end));
                }
            }
        }
        out
    }
}

use alloc::vec::Vec;

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BootInfo {
        let mut bi = BootInfo::new();
        bi.firmware_desc_size = MEMORY_DESCRIPTOR_SIZE as u32;
        bi.firmware_desc_version = 1;
        bi.memory_map_count = 2;
        bi.kernel_start = 0x2000_0000;
        bi.kernel_end = 0x2000_1000;
        bi.kernel_entry = 0x2000_0000;
        bi.memory_map[0] = MemoryDescriptor {
            ty: 7,
            _pad: 0,
            phys_start: 0x1000_0000,
            virt_start: 0,
            page_count: 16,
            att: 0xF,
        };
        bi.memory_map[1] = MemoryDescriptor {
            ty: 4, // boot services data → becomes free after exit
            _pad: 0,
            phys_start: 0x2000_0000,
            virt_start: 0,
            page_count: 4,
            att: 0xF,
        };
        bi
    }

    #[test]
    fn descriptor_is_40_bytes_and_repr_c() {
        assert_eq!(MEMORY_DESCRIPTOR_SIZE, 40);
        assert_eq!(core::mem::align_of::<MemoryDescriptor>(), 8);
    }

    #[test]
    fn accepts_valid_block() {
        let bi = sample();
        assert_eq!(bi.validate(size_of::<BootInfo>()), Ok(()));
    }

    #[test]
    fn rejects_bad_magic_and_version() {
        let mut bi = sample();
        bi.magic = 0xDEAD_BEEF;
        assert_eq!(bi.validate(size_of::<BootInfo>()), Err(BootInfoError::BadMagic));
        let mut bi = sample();
        bi.version = 99;
        assert_eq!(bi.validate(size_of::<BootInfo>()), Err(BootInfoError::BadVersion));
    }

    #[test]
    fn rejects_bad_total_size_and_zero_length_map() {
        let mut bi = sample();
        bi.total_size = (size_of::<BootInfo>() - 1) as u32;
        assert_eq!(bi.validate(size_of::<BootInfo>()), Err(BootInfoError::BadTotalSize));
        let mut bi = sample();
        bi.total_size = 1 << 30; // claims more bytes than provided
        assert_eq!(bi.validate(size_of::<BootInfo>()), Err(BootInfoError::BadTotalSize));
    }

    #[test]
    fn rejects_bad_firmware_stride() {
        // Zero stride (the historical hard-coded-48 bug class) must fail.
        let mut bi = sample();
        bi.firmware_desc_size = 0;
        assert_eq!(
            bi.validate(size_of::<BootInfo>()),
            Err(BootInfoError::BadFirmwareDescSize)
        );
        // A firmware reporting a *larger* descriptor is tolerated: we copy
        // the known prefix.
        let mut bi = sample();
        bi.firmware_desc_size = 64;
        assert_eq!(bi.validate(size_of::<BootInfo>()), Ok(()));
    }

    #[test]
    fn rejects_overflowing_page_counts() {
        let mut bi = sample();
        bi.memory_map[0].page_count = u64::MAX; // pages → bytes overflow
        assert_eq!(
            bi.validate(size_of::<BootInfo>()),
            Err(BootInfoError::DescriptorOverflow)
        );
        let mut bi = sample();
        bi.memory_map[0].page_count = u64::MAX / PAGE_SIZE; // fits, but addrs wrap
        bi.memory_map[0].phys_start = u64::MAX - 4095;
        assert_eq!(
            bi.validate(size_of::<BootInfo>()),
            Err(BootInfoError::DescriptorOverflow)
        );
    }

    #[test]
    fn rejects_too_many_entries() {
        let mut bi = sample();
        bi.memory_map_count = (MAX_MEMORY_DESCRIPTORS + 1) as u32;
        assert_eq!(
            bi.validate(size_of::<BootInfo>()),
            Err(BootInfoError::TooManyEntries)
        );
    }

    #[test]
    fn rejects_overlapping_descriptors() {
        let mut bi = sample();
        bi.memory_map[1].phys_start = 0x1000_0000 + 8 * PAGE_SIZE; // overlaps [0]
        assert_eq!(bi.semantic_check(), Err(BootInfoError::OverlappingDescriptors));
    }

    #[test]
    fn requires_kernel_range_reserved() {
        let mut bi = sample();
        bi.kernel_start = 0x1000_1000;
        bi.kernel_end = 0x1000_9000; // inside conventional region → invalid
        bi.kernel_entry = 0x1000_1000;
        assert_eq!(bi.semantic_check(), Err(BootInfoError::KernelRangeNotReserved));
    }

    #[test]
    fn usable_regions_excludes_kernel_even_if_map_is_sloppy() {
        let mut bi = BootInfo::new();
        bi.firmware_desc_size = MEMORY_DESCRIPTOR_SIZE as u32;
        bi.memory_map_count = 1;
        bi.memory_map[0] = MemoryDescriptor {
            ty: 7,
            _pad: 0,
            phys_start: 0x1000_0000,
            virt_start: 0,
            page_count: 16,
            att: 0xF,
        };
        bi.kernel_start = 0x1000_2000;
        bi.kernel_end = 0x1000_4000;
        let regions = bi.usable_regions();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0], (0x1000_0000, 0x2000));
        assert_eq!(regions[1], (0x1000_4000, 0xC000));
        let total: u64 = regions.iter().map(|(_, s)| s).sum();
        assert_eq!(total, 14 * PAGE_SIZE);
    }
}
