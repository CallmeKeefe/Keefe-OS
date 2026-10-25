//! Shared BootInfo ABI re-export. The canonical definition lives in the
//! boot-protocol crate and is used by both the UEFI loader and the kernel.

pub use keefeos_boot_protocol::bootinfo::*;
