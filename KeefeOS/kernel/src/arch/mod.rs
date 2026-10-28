//! x86_64 Architecture-Specific Code
//!
//! This module contains all architecture-specific implementations
//! for the x86_64 platform. The currently configured kernel target is x86_64 only. ARM64 and RISC-V
//! support are not implemented.

// The previous hand-written page-table manager does not compile against
// x86_64 0.15 and conflicts with memory/paging.rs. Keep it out of the default
// kernel until it is replaced by a single verified implementation.
#[cfg(feature = "experimental-paging")]
pub mod paging;

/// CPU privilege levels (rings)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PrivilegeLevel {
    /// Kernel mode - full hardware access
    Ring0 = 0,
    /// Reserved (not used in KeefeOS)
    Ring1 = 1,
    /// Reserved (not used in KeefeOS)
    Ring2 = 2,
    /// User mode - restricted access, capability-based permissions
    Ring3 = 3,
}

impl PrivilegeLevel {
    /// Returns true if this is a privileged level (kernel mode)
    #[inline]
    pub const fn is_privileged(self) -> bool {
        (self as u8) < 3
    }
    
    /// Returns true if this is user mode
    #[inline]
    pub const fn is_user(self) -> bool {
        self as u8 == 3
    }
}

/// Memory addresses used throughout the kernel
pub mod addresses {
    /// The physical address where the kernel is loaded by UEFI
    pub const KERNEL_LOAD_ADDR: usize = 0x100000; // 1 MB
    
    /// Higher-half kernel offset (0xFFFF800000000000)
    /// This places the kernel in the top 128 TB of virtual address space
    pub const KERNEL_OFFSET: usize = 0xFFFF800000000000;
    
    /// Calculate the higher-half virtual address for a physical address.
    ///
    /// This checks arithmetic overflow but does not prove that the virtual
    /// address is mapped in the current page tables.
    #[inline]
    pub const fn phys_to_virt(phys: usize) -> Option<usize> {
        phys.checked_add(KERNEL_OFFSET)
    }

    /// Calculate the physical offset for a higher-half kernel virtual address.
    ///
    /// Returns `None` for lower-half addresses instead of allowing subtraction
    /// to wrap. This does not validate an actual page-table mapping.
    #[inline]
    pub const fn virt_to_phys(virt: usize) -> Option<usize> {
        virt.checked_sub(KERNEL_OFFSET)
    }
}


#[cfg(test)]
mod address_conversion_tests {
    use super::addresses::{phys_to_virt, virt_to_phys, KERNEL_OFFSET};

    #[test]
    fn higher_half_conversion_round_trips() {
        let virtual_address = phys_to_virt(0x10_0000).expect("no overflow expected");
        assert_eq!(virtual_address, KERNEL_OFFSET + 0x10_0000);
        assert_eq!(virt_to_phys(virtual_address), Some(0x10_0000));
    }

    #[test]
    fn rejects_lower_half_address_and_addition_overflow() {
        assert_eq!(virt_to_phys(0x1000), None);
        assert_eq!(phys_to_virt(usize::MAX), None);
    }
}
