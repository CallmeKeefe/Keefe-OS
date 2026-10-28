//! Shared BootInfo ABI re-export and kernel-side handoff-address validation.
//! The canonical ABI definition lives in the boot-protocol crate and is used
//! by both the UEFI loader and the kernel.

pub use keefeos_boot_protocol::bootinfo::*;

/// Lowest accepted BootInfo physical address (leave the null/low guard area unused).
pub const BOOT_INFO_MIN_ADDRESS: u64 = 0x1000;
/// Exclusive upper bound of the identity-addressable BootInfo area during early boot.
pub const BOOT_INFO_MAX_ADDRESS_EXCLUSIVE: u64 = 0x4000_0000;

/// Validate the full BootInfo allocation range before dereferencing its pointer.
///
/// This checks numeric bounds, alignment, and overflow only. It relies on the
/// documented UEFI identity-mapping contract and does not independently prove
/// that a physical mapping exists in the current page tables.
pub fn is_valid_boot_info_pointer(address: u64) -> bool {
    let size = core::mem::size_of::<BootInfo>() as u64;
    let alignment = core::mem::align_of::<BootInfo>() as u64;
    address >= BOOT_INFO_MIN_ADDRESS
        && address % alignment == 0
        && address
            .checked_add(size)
            .is_some_and(|end| end <= BOOT_INFO_MAX_ADDRESS_EXCLUSIVE)
}

#[cfg(test)]
mod tests {
    use super::{
        is_valid_boot_info_pointer, BootInfo, BOOT_INFO_MAX_ADDRESS_EXCLUSIVE,
        BOOT_INFO_MIN_ADDRESS,
    };

    #[test]
    fn accepts_aligned_pointer_when_the_entire_structure_fits() {
        assert!(is_valid_boot_info_pointer(BOOT_INFO_MIN_ADDRESS));
    }

    #[test]
    fn rejects_pointer_below_low_memory_guard() {
        assert!(!is_valid_boot_info_pointer(BOOT_INFO_MIN_ADDRESS - 1));
    }

    #[test]
    fn rejects_misaligned_pointer() {
        let alignment = core::mem::align_of::<BootInfo>() as u64;
        assert!(!is_valid_boot_info_pointer(BOOT_INFO_MIN_ADDRESS + 1));
        assert!(alignment >= 1);
    }

    #[test]
    fn rejects_structure_that_crosses_upper_bound() {
        let size = core::mem::size_of::<BootInfo>() as u64;
        let alignment = core::mem::align_of::<BootInfo>() as u64;
        let last_valid = (BOOT_INFO_MAX_ADDRESS_EXCLUSIVE - size) & !(alignment - 1);
        assert!(is_valid_boot_info_pointer(last_valid));
        assert!(!is_valid_boot_info_pointer(last_valid + alignment));
    }

    #[test]
    fn rejects_address_addition_overflow() {
        assert!(!is_valid_boot_info_pointer(u64::MAX - 7));
    }
}
