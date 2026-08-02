//! KeefeOS UEFI Bootloader
//!
//! This bootloader uses the UEFI standard to:
//! 1. Initialize UEFI services
//! 2. Parse the memory map
//! 3. Load the kernel ELF binary
//! 4. Set up paging with identity mapping
//! 5. Jump to the kernel entry point

#![no_std]
#![no_main]

use uefi::prelude::*;
use uefi::proto::loaded_image::LoadedImage;
use uefi::table::boot::{LoadImageSource, MemoryType};
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{PageTable, PageTableFlags};

/// Kernel binary embedded in the bootloader (or loaded from disk)
/// In a real implementation, this would be loaded from the ESP partition
#[link_section = ".kernel"]
static KERNEL_BINARY: &[u8] = include_bytes!("../../target/x86_64-unknown-none/debug/keefeos-kernel");

/// UEFI application entry point
#[entry]
fn efi_main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize UEFI utilities
    uefi_services::init(&mut system_table).expect("Failed to initialize UEFI services");
    
    log::info!("========================================");
    log::info!("   KeefeOS Bootloader v{}", env!("CARGO_PKG_VERSION"));
    log::info!("========================================");
    
    // Get the memory map from UEFI
    let memory_map = system_table.boot_services().memory_map(MemoryType::LOADER_DATA);
    
    match memory_map {
        Ok((mut memory_map, _key)) => {
            log::info!("Memory map obtained: {} descriptors", memory_map.len());
            
            // Sort the memory map (required for some operations)
            memory_map.sort();
            
            // Count usable memory
            let mut total_usable = 0u64;
            for descriptor in memory_map.entries() {
                if descriptor.ty == MemoryType::CONVENTIONAL {
                    total_usable += descriptor.page_count * 4096;
                }
            }
            log::info!("Total usable RAM: {} MB", total_usable / (1024 * 1024));
            
            // Load the kernel
            let kernel_handle = load_kernel(system_table.boot_services());
            
            match kernel_handle {
                Ok(kernel_handle) => {
                    log::info!("Kernel loaded successfully");
                    
                    // Get kernel image info
                    let loaded_image = system_table
                        .boot_services()
                        .open_protocol::<LoadedImage>(
                            kernel_handle,
                            OpenProtocolParams {
                                handle: image_handle,
                                agent: image_handle,
                                controller: None,
                            },
                        );
                    
                    match loaded_image {
                        Ok(loaded_image) => {
                            let (kernel_base, kernel_size) = loaded_image.info();
                            let kernel_start = kernel_base as usize;
                            let kernel_end = kernel_start + kernel_size as usize;
                            
                            log::info!("Kernel image: {:#018x} - {:#018x} ({} bytes)",
                                       kernel_start, kernel_end, kernel_size);
                            
                            // Prepare to jump to kernel
                            // Exit boot services before jumping
                            let (boot_services, stale_handles) = 
                                system_table.into_boot_services();
                            
                            // We need to pass the memory map to the kernel
                            // The kernel expects: (memory_map_ptr, memory_map_size, kernel_start, kernel_end)
                            
                            // For now, we'll just call the kernel directly
                            // In production, you'd want proper handoff
                            
                            log::info!("Exiting boot services...");
                            
                            // Exit UEFI boot services
                            let mut system_table = boot_services.exit_boot_services(
                                image_handle,
                                &stale_handles,
                            ).expect("Failed to exit boot services");
                            
                            log::info!("Boot services exited. Handing control to kernel...");
                            
                            // Jump to kernel
                            // The kernel entry point signature:
                            // extern "C" fn kernel_start(
                            //     memory_map_ptr: usize,
                            //     memory_map_size: usize,
                            //     kernel_start: usize,
                            //     kernel_end: usize,
                            // ) -> !
                            
                            type KernelEntry = extern "C" fn(
                                usize, usize, usize, usize
                            ) -> !;
                            
                            let kernel_entry: KernelEntry = unsafe {
                                core::mem::transmute(kernel_base)
                            };
                            
                            // Call the kernel
                            kernel_entry(
                                memory_map.entries().as_ptr() as usize,
                                memory_map.len() * core::mem::size_of::<uefi::table::boot::MemoryDescriptor>(),
                                kernel_start,
                                kernel_end,
                            )
                        }
                        Err(e) => {
                            log::error!("Failed to open LoadedImage protocol: {:?}", e);
                            Status::ABORTED
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to load kernel: {:?}", e);
                    Status::LOAD_ERROR
                }
            }
        }
        Err(e) => {
            log::error!("Failed to get memory map: {:?}", e);
            Status::ABORTED
        }
    }
}

/// Load the kernel from the embedded binary or from disk
fn load_kernel(boot_services: &uefi::table::boot::BootServices) -> Result<Handle, Status> {
    log::info!("Loading kernel binary...");
    
    // Create a load image source from the embedded binary
    let source = LoadImageSource::from_buffer(KERNEL_BINARY.as_ptr(), KERNEL_BINARY.len());
    
    // Load the kernel image
    match boot_services.load_image(false, Handle::NULL, source, None) {
        Ok(handle) => {
            log::info!("Kernel image loaded at handle {:?}", handle);
            Ok(handle)
        }
        Err(status) => {
            log::error!("Failed to load kernel image: {:?}", status);
            Err(status)
        }
    }
}

/// Panic handler for the bootloader
#[panic_handler]
fn bootloader_panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("Bootloader panic: {}", info);
    
    loop {
        x86_64::instructions::hlt();
    }
}
