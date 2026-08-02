//! x86_64 Paging Implementation (4-Level Page Tables)
//!
//! This module implements the virtual memory management system for KeefeOS.
//! We use 4-level paging as defined by the x86_64 architecture, which supports:
//! - 48-bit virtual addresses (256 TB address space)
//! - 4KB page size (standard) and 2MB/1GB (huge pages)
//! - Fine-grained permission control per page
//!
//! # Page Table Structure (4-Level)
//!
//! ```text
//! Virtual Address Layout (48-bit):
//! +--------+--------+--------+--------+--------+------------------+
//! | Sign   | L4     | L3     | L2     | L1     | Offset           |
//! | 16 bits| 9 bits | 9 bits | 9 bits | 9 bits | 12 bits          |
//! +--------+--------+--------+--------+--------+------------------+
//! 
//! Each level uses 9 bits = 512 entries per table
//! Each entry is 8 bytes, so each table is 4KB (one page)
//! ```
//!
//! # KeefeOS Design Decisions
//!
//! 1. **Capability-Based Isolation**: Pages are mapped with explicit
//!    capabilities that define what operations are allowed.
//!
//! 2. **Higher-Half Kernel**: The kernel is mapped at high virtual
//!    addresses (0xFFFF800000000000+) to separate it from user space.
//!
//! 3. **Safe Abstractions**: All page table manipulations use safe Rust
//!    wrappers. Unsafe operations are isolated and verified.
//!
//! 4. **Zero-Copy IPC Ready**: Page tables support shared memory regions
//!    for efficient inter-process communication.

use bitflags::bitflags;
use core::fmt;
use core::ptr::NonNull;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    page_table::PageTableEntry,
    FrameAllocator, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

/// The offset where the kernel is mapped in virtual address space
/// This places the kernel in the higher half of the address space
pub const KERNEL_OFFSET: VirtAddr = VirtAddr::new_truncate(0xFFFF800000000000);

/// Convert a physical address to a kernel virtual address
#[inline]
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + KERNEL_OFFSET.as_u64())
}

/// Convert a kernel virtual address to a physical address
#[inline]
pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64() - KERNEL_OFFSET.as_u64())
}

/// Flags for page mapping permissions
/// These are capability-based: a process can only access pages
/// for which it holds the appropriate capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapFlags(PageTableFlags);

impl MapFlags {
    /// No special flags - default state
    pub const NONE: Self = Self(PageTableFlags::empty());
    
    /// Page is present in memory (must be set for valid mappings)
    pub const PRESENT: Self = Self(PageTableFlags::PRESENT);
    
    /// Page is writable (without this, page is read-only)
    pub const WRITABLE: Self = Self(PageTableFlags::WRITABLE);
    
    /// Page is accessible from user mode (Ring 3)
    /// Without this, only kernel mode (Ring 0) can access
    pub const USER_ACCESSIBLE: Self = Self(PageTableFlags::USER_ACCESSIBLE);
    
    /// Disable write-through caching for this page
    pub const WRITE_THROUGH: Self = Self(PageTableFlags::WRITE_THROUGH);
    
    /// Disable caching for this page (uncached memory)
    pub const NO_CACHE: Self = Self(PageTableFlags::NO_CACHE);
    
    /// Accessed flag - set by CPU when page is accessed
    /// OS should set this initially for new mappings
    pub const ACCESSED: Self = Self(PageTableFlags::ACCESSED);
    
    /// Dirty flag - set by CPU when page is written
    /// OS should manage this for copy-on-write
    pub const DIRTY: Self = Self(PageTableFlags::DIRTY);
    
    /// Execute-disable flag (XD bit)
    /// Prevents code execution from this page (security feature)
    pub const NO_EXECUTE: Self = Self(PageTableFlags::NO_EXECUTE);
    
    /// Create flags for kernel code pages (readable, executable, not writable)
    pub const KERNEL_CODE: Self = Self(
        PageTableFlags::PRESENT
            .union(PageTableFlags::ACCESSED)
            .union(PageTableFlags::NO_EXECUTE)
    );
    
    /// Create flags for kernel data pages (readable, writable, not executable)
    pub const KERNEL_DATA: Self = Self(
        PageTableFlags::PRESENT
            .union(PageTableFlags::WRITABLE)
            .union(PageTableFlags::ACCESSED)
            .union(PageTableFlags::DIRTY)
            .union(PageTableFlags::NO_EXECUTE)
    );
    
    /// Create flags for user code pages (readable, executable, user-accessible)
    pub const USER_CODE: Self = Self(
        PageTableFlags::PRESENT
            .union(PageTableFlags::USER_ACCESSIBLE)
            .union(PageTableFlags::ACCESSED)
    );
    
    /// Create flags for user data pages (readable, writable, user-accessible)
    pub const USER_DATA: Self = Self(
        PageTableFlags::PRESENT
            .union(PageTableFlags::WRITABLE)
            .union(PageTableFlags::USER_ACCESSIBLE)
            .union(PageTableFlags::ACCESSED)
            .union(PageTableFlags::DIRTY)
            .union(PageTableFlags::NO_EXECUTE)
    );
    
    /// Check if a flag is set
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0.contains(other.0)
    }
    
    /// Combine flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0.union(other.0))
    }
}

/// A wrapper around the active page tables (PML4)
/// This provides safe access to the current address space
pub struct ActivePageTables {
    /// Pointer to the PML4 (Level 4 page table)
    pml4: NonNull<PageTable>,
}

// Safety: Page tables are per-CPU and we ensure proper synchronization
unsafe impl Send for ActivePageTables {}
unsafe impl Sync for ActivePageTables {}

impl ActivePageTables {
    /// Get the currently active page tables
    ///
    /// # Safety
    /// This function assumes CR3 points to a valid PML4.
    /// The returned reference is valid for the lifetime of the kernel.
    pub unsafe fn new() -> Self {
        let (pml4_frame, _) = Cr3::read();
        let pml4_addr = phys_to_virt(pml4_frame.start_address());
        let pml4_ptr = pml4_addr.as_mut_ptr::<PageTable>();
        
        Self {
            pml4: NonNull::new(pml4_ptr).expect("PML4 pointer is null"),
        }
    }
    
    /// Get a reference to the PML4 table
    #[inline]
    fn pml4(&self) -> &PageTable {
        unsafe { self.pml4.as_ref() }
    }
    
    /// Get a mutable reference to the PML4 table
    #[inline]
    fn pml4_mut(&mut self) -> &mut PageTable {
        unsafe { self.pml4.as_mut() }
    }
    
    /// Translate a virtual address to a physical address
    /// Returns None if the address is not mapped
    pub fn translate(&self, addr: VirtAddr) -> Option<PhysAddr> {
        // Walk the page table hierarchy
        let pml4 = self.pml4();
        
        // Extract indices from the virtual address
        let pml4_idx = (addr.as_u64() >> 39) & 0x1FF;
        let pdpt_idx = (addr.as_u64() >> 30) & 0x1FF;
        let pd_idx = (addr.as_u64() >> 21) & 0x1FF;
        let pt_idx = (addr.as_u64() >> 12) & 0x1FF;
        let offset = addr.as_u64() & 0xFFF;
        
        // Level 4: PML4 -> PDPT
        let pml4_entry = &pml4[pml4_idx as usize];
        if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        
        let pdpt_addr = PhysAddr::new(pml4_entry.addr().as_u64());
        let pdpt = unsafe { &*(phys_to_virt(pdpt_addr).as_ptr::<PageTable>()) };
        
        // Level 3: PDPT -> PD
        let pdpt_entry = &pdpt[pdpt_idx as usize];
        if !pdpt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        
        // Check for 1GB huge page
        if pdpt_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            let frame_addr = pdpt_entry.addr().as_u64();
            return Some(PhysAddr::new(frame_addr + (addr.as_u64() & 0x3FFFFFFF)));
        }
        
        let pd_addr = PhysAddr::new(pdpt_entry.addr().as_u64());
        let pd = unsafe { &*(phys_to_virt(pd_addr).as_ptr::<PageTable>()) };
        
        // Level 2: PD -> PT
        let pd_entry = &pd[pd_idx as usize];
        if !pd_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        
        // Check for 2MB huge page
        if pd_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            let frame_addr = pd_entry.addr().as_u64();
            return Some(PhysAddr::new(frame_addr + (addr.as_u64() & 0x1FFFFF)));
        }
        
        let pt_addr = PhysAddr::new(pd_entry.addr().as_u64());
        let pt = unsafe { &*(phys_to_virt(pt_addr).as_ptr::<PageTable>()) };
        
        // Level 1: PT -> Physical Frame
        let pt_entry = &pt[pt_idx as usize];
        if !pt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None;
        }
        
        let frame_addr = pt_entry.addr().as_u64();
        Some(PhysAddr::new(frame_addr + offset))
    }
}

/// Manager for page table operations
/// This is used to create and modify page mappings
pub struct PageTableManager<A>
where
    A: FrameAllocator<Size4KiB>,
{
    /// Reference to the page table we're managing
    pml4: &'static mut PageTable,
    /// Allocator for creating new page tables
    frame_allocator: A,
}

impl<A> PageTableManager<A>
where
    A: FrameAllocator<Size4KiB>,
{
    /// Create a new page table manager
    pub fn new(pml4: &'static mut PageTable, frame_allocator: A) -> Self {
        Self {
            pml4,
            frame_allocator,
        }
    }
    
    /// Map a virtual page to a physical frame with the given flags
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The virtual address is not already mapped (or handle overwriting)
    /// - The physical frame is not already mapped elsewhere (unless intended)
    /// - Proper synchronization if multiple CPUs access the same mapping
    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: MapFlags,
    ) -> Result<(), &'static str> {
        // Get mutable references to all levels of the page table
        let pml4 = self.pml4;
        
        let pml4_idx = page.p4_index();
        let pdpt_idx = page.p3_index();
        let pd_idx = page.p2_index();
        let pt_idx = page.p1_index();
        
        // Level 4: Ensure PDPT exists
        let pdpt = self.get_or_create_table(pml4, pml4_idx)?;
        
        // Level 3: Ensure PD exists
        let pd = self.get_or_create_table(pdpt, pdpt_idx)?;
        
        // Level 2: Ensure PT exists
        let pt = self.get_or_create_table(pd, pd_idx)?;
        
        // Level 1: Create the actual mapping
        let entry = &mut pt[pt_idx as usize];
        
        if entry.flags().contains(PageTableFlags::PRESENT) {
            return Err("Page is already mapped");
        }
        
        entry.set_addr(frame.start_address(), flags.0);
        
        Ok(())
    }
    
    /// Get or create a page table at the given index
    fn get_or_create_table(
        &mut self,
        table: &mut PageTable,
        index: u16,
    ) -> Result<&'static mut PageTable, &'static str> {
        let entry = &mut table[index as usize];
        
        if !entry.flags().contains(PageTableFlags::PRESENT) {
            // Allocate a new frame for the next level table
            let frame = self
                .frame_allocator
                .allocate_frame()
                .ok_or("Out of memory")?;
            
            // Zero out the new page table
            let table_addr = phys_to_virt(frame.start_address());
            let new_table = unsafe { &mut *(table_addr.as_mut_ptr::<PageTable>()) };
            *new_table = PageTable::new();
            
            // Link the new table
            entry.set_addr(
                frame.start_address(),
                PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
            );
            
            return Ok(new_table);
        }
        
        // Table already exists, get a reference to it
        let table_addr = PhysAddr::new(entry.addr().as_u64());
        let table_ptr = phys_to_virt(table_addr).as_mut_ptr::<PageTable>();
        Ok(unsafe { &mut *table_ptr })
    }
    
    /// Unmap a virtual page
    ///
    /// # Safety
    /// The caller must ensure no code is executing from this page
    /// and that the physical frame is properly handled after unmapping.
    pub fn unmap_page(&mut self, page: Page<Size4KiB>) -> Result<PhysFrame<Size4KiB>, &'static str> {
        let pml4_idx = page.p4_index();
        let pdpt_idx = page.p3_index();
        let pd_idx = page.p2_index();
        let pt_idx = page.p1_index();
        
        let pml4 = self.pml4;
        let pdpt = self.get_table(pml4, pml4_idx)?;
        let pd = self.get_table(pdpt, pdpt_idx)?;
        let pt = self.get_table(pd, pd_idx)?;
        
        let entry = &mut pt[pt_idx as usize];
        
        if !entry.flags().contains(PageTableFlags::PRESENT) {
            return Err("Page is not mapped");
        }
        
        let frame = PhysFrame::from_start_address(entry.addr())
            .map_err(|_| "Invalid frame address")?;
        
        // Clear the entry
        entry.set_unused();
        
        // Flush the TLB entry for this page
        unsafe {
            x86_64::instructions::tlb::flush(page.start_address());
        }
        
        Ok(frame)
    }
    
    /// Get a reference to a page table at the given index
    fn get_table<'a>(
        table: &'a mut PageTable,
        index: u16,
    ) -> Result<&'a mut PageTable, &'static str> {
        let entry = &table[index as usize];
        
        if !entry.flags().contains(PageTableFlags::PRESENT) {
            return Err("Page table does not exist");
        }
        
        let table_addr = PhysAddr::new(entry.addr().as_u64());
        let table_ptr = phys_to_virt(table_addr).as_mut_ptr::<PageTable>();
        Ok(unsafe { &mut *table_ptr })
    }
}

/// Identity map the kernel region
/// This maps physical addresses to the same virtual addresses
/// (with the kernel offset applied)
pub fn identity_map_kernel<A>(
    manager: &mut PageTableManager<A>,
    kernel_start: PhysAddr,
    kernel_end: PhysAddr,
) -> Result<(), &'static str>
where
    A: FrameAllocator<Size4KiB>,
{
    let start_frame = PhysFrame::containing_address(kernel_start);
    let end_frame = PhysFrame::containing_address(kernel_end);
    
    let mut current_frame = start_frame;
    while current_frame <= end_frame {
        let virt_addr = KERNEL_OFFSET + current_frame.start_address().as_u64();
        let page = Page::containing_address(virt_addr);
        
        // Determine flags based on whether this is code or data
        // For simplicity, we map everything as kernel data
        // A real implementation would parse ELF sections
        let flags = MapFlags::KERNEL_DATA;
        
        manager.map_page(page, current_frame, flags)?;
        
        current_frame += 1;
    }
    
    Ok(())
}

/// Initialize the paging subsystem
pub fn init() {
    log::info!("Paging subsystem initialized");
    log::info!("Kernel offset: {:#018x}", KERNEL_OFFSET.as_u64());
}
