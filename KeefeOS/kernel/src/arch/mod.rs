//! x86_64 Architecture-Specific Code
//!
//! This module contains all architecture-specific implementations
//! for the x86_64 platform. To support other architectures (ARM64, RISC-V),
//! implement equivalent modules with the same interface.

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
    
    /// Convert a physical address to a virtual address in kernel space
    #[inline]
    pub const fn phys_to_virt(phys: usize) -> usize {
        phys + KERNEL_OFFSET
    }
    
    /// Convert a virtual address in kernel space to a physical address
    #[inline]
    pub const fn virt_to_phys(virt: usize) -> usize {
        virt - KERNEL_OFFSET
    }
}
