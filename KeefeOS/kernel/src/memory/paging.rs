//! Virtual Memory Manager (Paging) for x86_64
//!
//! This module implements 4-level paging as defined by the x86_64 architecture.
//! It provides safe Rust abstractions for page tables and virtual memory mapping.
//!
//! # x86_64 4-Level Paging Structure
//!
//! ```text
//! Virtual Address Layout (48-bit canonical addresses):
//! +--------+--------+--------+--------+--------+------------------+
//! | Sign   | L4     | L3     | L2     | L1     | Offset           |
//! | 16 bits| 9 bits | 9 bits | 9 bits | 9 bits | 12 bits          |
//! |        | (PML4) | (PDPT) | (PD)   | (PT)   | (within page)    |
//! +--------+--------+--------+--------+--------+------------------+
//! 63      48 47    39 38    30 29    21 20    12 11             0
//!
//! Each level uses 9 bits = 512 entries per table
//! Each entry is 8 bytes, so each table fits exactly in one 4KB page
//! ```
//!
//! # Page Table Entry (PTE) Format (64-bit)
//!
//! ```text
//! +-----+-----+-----+-----+-----+------+-----+-----+-------------+-----+
//! | XD  | ... | PAT | PS  | G   | A    | D   | U/W | PWT | PCD | PRE |
//! | 63  | ... | 12  | 7   | 6   | 5    | 4   | 3   | 2   | 1   | 0   |
//! +-----+-----+-----+-----+-----+------+-----+-----+-----+-----+-----+
//!
//! Bit 0 (PRE): Present - Page is in memory
//! Bit 1 (U/W): User/Supervisor - 0=kernel only, 1=user accessible
//! Bit 2 (PWT): Write-Through - Controls caching strategy
//! Bit 3 (PCD): Cache Disable - Disable caching for this page
//! Bit 4 (D): Dirty - CPU sets when writing (software must manage)
//! Bit 5 (A): Accessed - CPU sets when reading/writing
//! Bit 6 (G): Global - Not flushed from TLB on CR3 write
//! Bit 7 (PS): Page Size - 1=huge page (2MB/1GB), 0=4KB page
//! Bit 12 (PAT): Page Attribute Table - Extended caching control
//! Bit 63 (XD): Execute Disable - Prevent code execution (security)
//! ```
//!
//! # Capability-Based Security Design
//!
//! In KeefeOS, memory access is controlled through capabilities:
//! - A process can only map pages it holds capabilities for
//! - Page permissions (R/W/X) are enforced at the hardware level
//! - User/supervisor bit provides isolation between kernel and user space
//! - The XD bit prevents code injection attacks (NX protection)
//!
//! # Memory Layout Strategy
//!
//! We use a higher-half kernel design:
//! - User space: 0x0000_0000_0000_0000 to 0x0000_7FFF_FFFF_FFFF (128 TB)
//! - Kernel space: 0xFFFF_8000_0000_0000 to 0xFFFF_FFFF_FFFF_FFFF (128 TB)
//!
//! This provides:
//! - Clear separation between user and kernel memory
//! - Protection against accidental kernel pointer dereferences in user mode
//! - Easy identity mapping of physical memory at KERNEL_OFFSET


use bitflags::bitflags;
use core::fmt;
use core::ptr::NonNull;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    page_table::PageTableEntry,
    FrameAllocator, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

/// The offset where the kernel is mapped in virtual address space.
///
/// This places the kernel in the higher half of the address space,
/// specifically in the top 128 TB of the 48-bit virtual address space.
///
/// # Why This Offset?
///
/// The value 0xFFFF800000000000 is chosen because:
/// 1. It's a canonical address (bits 48-63 are sign extension of bit 47)
/// 2. It leaves the entire lower half (128 TB) for user-space processes
/// 3. It provides a simple arithmetic conversion: virt = phys + OFFSET
/// 4. Any NULL pointer dereference (0x0) will fault, not access kernel memory
pub const KERNEL_OFFSET: VirtAddr = VirtAddr::new_truncate(0xFFFF800000000000);

/// Convert a physical address to a kernel virtual address.
///
/// This is used to access physical memory from the kernel.
/// All physical frames are identity-mapped at KERNEL_OFFSET.
///
/// # Example
/// ```
/// let phys = PhysAddr::new(0x100000); // 1 MB
/// let virt = phys_to_virt(phys);      // 0xFFFF800000100000
/// ```
#[inline]
pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    VirtAddr::new(phys.as_u64() + KERNEL_OFFSET.as_u64())
}

/// Convert a kernel virtual address to a physical address.
///
/// This is the inverse of `phys_to_virt`, used when we need to store
/// physical addresses in page table entries or pass them to hardware.
///
/// # Safety
/// The input must be a kernel virtual address (>= KERNEL_OFFSET).
/// Passing a user-space address will result in an invalid physical address.
#[inline]
pub fn virt_to_phys(virt: VirtAddr) -> PhysAddr {
    PhysAddr::new(virt.as_u64() - KERNEL_OFFSET.as_u64())
}

bitflags! {
    /// Page mapping flags for capability-based memory access control.
    ///
    /// These flags directly correspond to x86_64 page table entry bits,
    /// providing hardware-enforced memory protection.
    ///
    /// # Capability-Based Security
    ///
    /// In a capability-based system, these flags represent the "authority"
    /// a process has over a memory region:
    /// - READ capability: implied by any mapping (hardware doesn't have read-only flag)
    /// - WRITE capability: WRITABLE flag
    /// - EXECUTE capability: absence of NO_EXECUTE flag
    /// - USER capability: USER_ACCESSIBLE flag
    ///
    /// A process can only modify these flags if it holds the appropriate
    /// capability (in practice, only the kernel can modify page tables).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapFlags: u64 {
        /// No special flags - minimal permissions
        const NONE = 0;

        /// Page is present in memory (must be set for valid mappings).
        ///
        /// If this flag is clear, accessing the page causes a page fault (#PF).
        /// The OS handles page faults to implement demand paging or detect errors.
        const PRESENT = PageTableFlags::PRESENT.bits();

        /// Page is writable (without this, page is read-only).
        ///
        /// # Security Implications
        /// - Code pages should NOT have this flag (prevent self-modifying code)
        /// - Data pages should have this flag for normal operation
        /// - Read-only data (constants, etc.) should NOT have this flag
        /// - Combined with USER_ACCESSIBLE, allows user-space writes
        const WRITABLE = PageTableFlags::WRITABLE.bits();

        /// Page is accessible from user mode (Ring 3).
        ///
        /// # Capability-Based Isolation
        /// Without this flag, only kernel mode (Ring 0) can access the page.
        /// This is the primary mechanism for isolating kernel memory from user processes.
        ///
        /// # Security Best Practices
        /// - Kernel code/data: NEVER set this flag
        /// - User code/data: ALWAYS set this flag
        /// - Shared memory: Set this flag with appropriate WRITABLE controls
        const USER_ACCESSIBLE = PageTableFlags::USER_ACCESSIBLE.bits();

        /// Disable write-through caching for this page.
        ///
        /// When set with NO_CACHE, provides uncached memory access.
        /// Used for memory-mapped I/O regions where caching would cause issues.
        const WRITE_THROUGH = PageTableFlags::WRITE_THROUGH.bits();

        /// Disable caching for this page (uncached memory).
        ///
        /// # Use Cases
        /// - Memory-mapped I/O (MMIO) devices
        /// - Framebuffer memory
        /// - DMA buffers that must be coherent with devices
        ///
        /// # Performance Warning
        /// Uncached memory is significantly slower than cached memory.
        /// Only use when necessary for device interaction.
        const NO_CACHE = PageTableFlags::NO_CACHE.bits();

        /// Accessed flag - set by CPU when page is accessed (read or write).
        ///
        /// # OS Responsibility
        /// The OS should set this initially for new mappings.
        /// The OS may clear this to track page usage for:
        /// - Page replacement algorithms
        /// - Working set estimation
        /// - Memory profiling
        const ACCESSED = PageTableFlags::ACCESSED.bits();

        /// Dirty flag - set by CPU when page is written.
        ///
        /// # OS Responsibility
        /// The OS must set this for writable pages.
        /// This flag is critical for:
        /// - Copy-on-write (COW) implementations
        /// - Page swapping (only dirty pages need to be written back)
        /// - Memory deduplication
        const DIRTY = PageTableFlags::DIRTY.bits();

        /// Execute-disable flag (XD bit / NX bit).
        ///
        /// # Security Feature
        /// When set, prevents code execution from this page.
        /// This is a critical security feature that prevents:
        /// - Buffer overflow exploits (can't execute injected code)
        /// - Return-oriented programming (ROP) from data pages
        /// - JIT spraying attacks
        ///
        /// # Best Practices
        /// - Data pages: ALWAYS set NO_EXECUTE
        /// - Code pages: Never set NO_EXECUTE (allow execution)
        /// - Stack pages: ALWAYS set NO_EXECUTE (prevent stack-based exploits)
        const NO_EXECUTE = PageTableFlags::NO_EXECUTE.bits();

        /// Global flag - page is not flushed from TLB on CR3 switch.
        ///
        /// # Performance Optimization
        /// Global pages remain in the TLB across context switches,
        /// reducing TLB misses for frequently accessed kernel pages.
        ///
        /// # Usage
        /// Only for kernel pages that are always mapped (like the kernel itself).
        /// Never use for user-space pages or per-process kernel mappings.
        const GLOBAL = PageTableFlags::GLOBAL.bits();

        /// Huge page flag (2MB or 1GB pages).
        ///
        /// When set in a PDPTE, creates a 1GB huge page.
        /// When set in a PDE, creates a 2MB huge page.
        ///
        /// # Benefits
        /// - Reduced TLB pressure (one entry covers more memory)
        /// - Faster page table walks (fewer levels to traverse)
        /// - Better performance for large contiguous allocations
        ///
        /// # Drawbacks
        /// - Internal fragmentation (must allocate full 2MB/1GB)
        /// - Less granular permission control
        const HUGE_PAGE = PageTableFlags::HUGE_PAGE.bits();
    }
}

impl MapFlags {
    /// Create flags for kernel code pages.
    ///
    /// Kernel code should be:
    /// - Present and accessible (by kernel only)
    /// - Readable and executable
    /// - NOT writable (prevent self-modification)
    /// - Cached for performance
    /// - Marked as global (always mapped)
    pub const KERNEL_CODE: Self = Self::PRESENT
        .union(Self::ACCESSED)
        .union(Self::GLOBAL);

    /// Create flags for kernel data pages.
    ///
    /// Kernel data should be:
    /// - Present and accessible (by kernel only)
    /// - Readable and writable
    /// - NOT executable (prevent code injection)
    /// - Cached for performance
    /// - Marked as dirty (will be modified)
    /// - Marked as global (always mapped)
    pub const KERNEL_DATA: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::ACCESSED)
        .union(Self::DIRTY)
        .union(Self::NO_EXECUTE)
        .union(Self::GLOBAL);

    /// Create flags for user code pages.
    ///
    /// User code should be:
    /// - Present and accessible (by user and kernel)
    /// - Readable and executable
    /// - NOT writable (prevent self-modification)
    /// - Cached for performance
    pub const USER_CODE: Self = Self::PRESENT
        .union(Self::USER_ACCESSIBLE)
        .union(Self::ACCESSED);

    /// Create flags for user data pages.
    ///
    /// User data should be:
    /// - Present and accessible (by user and kernel)
    /// - Readable and writable
    /// - NOT executable (prevent code injection - DEP/NX)
    /// - Cached for performance
    /// - Marked as dirty (will be modified)
    pub const USER_DATA: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::USER_ACCESSIBLE)
        .union(Self::ACCESSED)
        .union(Self::DIRTY)
        .union(Self::NO_EXECUTE);

    /// Create flags for user stack pages.
    ///
    /// User stacks require special handling:
    /// - Writable (obviously, for local variables)
    /// - NOT executable (critical security feature)
    /// - User accessible
    ///
    /// Note: Guard pages (unmapped pages below stack) provide
    /// overflow detection. Those should have no flags.
    pub const USER_STACK: Self = Self::USER_DATA;

    /// Create flags for shared memory pages.
    ///
    /// Shared memory between processes:
    /// - User accessible (all participating processes)
    /// - Optionally writable (depends on sharing semantics)
    /// - NOT executable (security best practice)
    pub const SHARED_MEMORY: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::USER_ACCESSIBLE)
        .union(Self::ACCESSED)
        .union(Self::DIRTY)
        .union(Self::NO_EXECUTE);

    /// Create flags for MMIO (Memory-Mapped I/O) regions.
    ///
    /// MMIO requires special caching behavior:
    /// - Uncached (UC) to ensure ordered access
    /// - NOT executable (device memory, not code)
    /// - Usually kernel-only (devices are kernel-managed)
    pub const MMIO: Self = Self::PRESENT
        .union(Self::WRITABLE)
        .union(Self::NO_CACHE)
        .union(Self::NO_EXECUTE);
}

impl From<MapFlags> for PageTableFlags {
    fn from(flags: MapFlags) -> Self {
        PageTableFlags::from_bits_truncate(flags.bits())
    }
}

/// A wrapper around the active page tables (PML4).
///
/// This provides safe access to the current address space's page tables.
/// The PML4 (Page Map Level 4) is the root of the 4-level page table hierarchy.
///
/// # Thread Safety
///
/// This type is `Send` and `Sync` because:
/// - Page tables are per-address-space, not per-CPU
/// - We assume proper synchronization at a higher level (e.g., per-process lock)
/// - The actual page table memory is accessed through kernel virtual addresses
///
/// # Capability Model
///
/// Holding an `ActivePageTables` reference grants the capability to:
/// - Query the current address space mappings
/// - Translate virtual addresses
/// It does NOT grant the capability to modify mappings (use `PageTableManager` for that).
pub struct ActivePageTables {
    /// Pointer to the PML4 (Level 4 page table).
    ///
    /// The PML4 is always mapped at KERNEL_OFFSET + PML4_PHYSICAL_ADDR.
    /// We use a NonNull pointer to ensure it's never null and enable niche optimization.
    pml4: NonNull<PageTable>,
}

// Safety: Page tables are properly synchronized at the address-space level.
// Multiple CPUs can read the same page tables concurrently.
unsafe impl Send for ActivePageTables {}
unsafe impl Sync for ActivePageTables {}

impl ActivePageTables {
    /// Get the currently active page tables.
    ///
    /// This reads the CR3 register to find the current PML4 physical address,
    /// then converts it to a kernel virtual address for safe access.
    ///
    /// # Safety
    ///
    /// This function assumes:
    /// - CR3 points to a valid PML4 (guaranteed after proper initialization)
    /// - The PML4 is identity-mapped at KERNEL_OFFSET
    /// - The returned reference is valid for the lifetime of the kernel
    ///
    /// # Returns
    ///
    /// An `ActivePageTables` instance representing the current address space.
    pub unsafe fn new() -> Self {
        // Read CR3 to get the current PML4 frame
        let (pml4_frame, _) = Cr3::read();
        let pml4_addr = pml4_frame.start_address();

        // Convert to kernel virtual address
        let pml4_virt = phys_to_virt(pml4_addr);
        let pml4_ptr = pml4_virt.as_mut_ptr::<PageTable>();

        Self {
            pml4: NonNull::new(pml4_ptr).expect("PML4 pointer must not be null"),
        }
    }

    /// Get an immutable reference to the PML4 table.
    #[inline]
    fn pml4(&self) -> &PageTable {
        unsafe { self.pml4.as_ref() }
    }

    /// Get a mutable reference to the PML4 table.
    ///
    /// # Safety
    ///
    /// This allows modification of the active page tables.
    /// Caller must ensure proper synchronization if multiple CPUs
    /// might modify the page tables concurrently.
    #[inline]
    fn pml4_mut(&mut self) -> &mut PageTable {
        unsafe { self.pml4.as_mut() }
    }

    /// Translate a virtual address to a physical address.
    ///
    /// This performs a software page table walk, following the same
    /// algorithm the CPU's MMU uses for hardware translation.
    ///
    /// # Algorithm
    ///
    /// 1. Extract indices from virtual address (L4, L3, L2, L1)
    /// 2. Walk PML4 -> PDPT -> PD -> PT
    /// 3. Check PRESENT flag at each level
    /// 4. Handle huge pages (1GB at L3, 2MB at L2)
    /// 5. Combine frame address with page offset
    ///
    /// # Returns
    ///
    /// - `Some(PhysAddr)` if the address is mapped
    /// - `None` if any level is not present (address not mapped)
    ///
    /// # Complexity
    ///
    /// O(1) - exactly 4 memory accesses worst case (fewer for huge pages)
    pub fn translate(&self, addr: VirtAddr) -> Option<PhysAddr> {
        let pml4 = self.pml4();

        // Extract indices from the virtual address
        // These correspond to the 9-bit fields in the x86_64 virtual address
        let pml4_idx = (addr.as_u64() >> 39) & 0x1FF; // Bits 39-47
        let pdpt_idx = (addr.as_u64() >> 30) & 0x1FF; // Bits 30-38
        let pd_idx = (addr.as_u64() >> 21) & 0x1FF;   // Bits 21-29
        let pt_idx = (addr.as_u64() >> 12) & 0x1FF;   // Bits 12-20
        let offset = addr.as_u64() & 0xFFF;           // Bits 0-11

        // Level 4: PML4 -> PDPT (Page Directory Pointer Table)
        let pml4_entry = &pml4[pml4_idx as usize];
        if !pml4_entry.flags().contains(PageTableFlags::PRESENT) {
            return None; // PML4 entry not present
        }

        let pdpt_addr = PhysAddr::new(pml4_entry.addr().as_u64());
        let pdpt = unsafe { &*(phys_to_virt(pdpt_addr).as_ptr::<PageTable>()) };

        // Level 3: PDPT -> PD (Page Directory)
        let pdpt_entry = &pdpt[pdpt_idx as usize];
        if !pdpt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None; // PDPT entry not present
        }

        // Check for 1GB huge page (PS flag in PDPTE)
        if pdpt_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            // 1GB page: combine 30-bit frame address with 30-bit offset
            let frame_addr = pdpt_entry.addr().as_u64();
            let phys = frame_addr + (addr.as_u64() & 0x3FFFFFFF);
            return Some(PhysAddr::new(phys));
        }

        let pd_addr = PhysAddr::new(pdpt_entry.addr().as_u64());
        let pd = unsafe { &*(phys_to_virt(pd_addr).as_ptr::<PageTable>()) };

        // Level 2: PD -> PT (Page Table)
        let pd_entry = &pd[pd_idx as usize];
        if !pd_entry.flags().contains(PageTableFlags::PRESENT) {
            return None; // PD entry not present
        }

        // Check for 2MB huge page (PS flag in PDE)
        if pd_entry.flags().contains(PageTableFlags::HUGE_PAGE) {
            // 2MB page: combine 21-bit frame address with 21-bit offset
            let frame_addr = pd_entry.addr().as_u64();
            let phys = frame_addr + (addr.as_u64() & 0x1FFFFF);
            return Some(PhysAddr::new(phys));
        }

        let pt_addr = PhysAddr::new(pd_entry.addr().as_u64());
        let pt = unsafe { &*(phys_to_virt(pt_addr).as_ptr::<PageTable>()) };

        // Level 1: PT -> Physical Frame
        let pt_entry = &pt[pt_idx as usize];
        if !pt_entry.flags().contains(PageTableFlags::PRESENT) {
            return None; // PT entry not present
        }

        // Standard 4KB page: combine frame address with 12-bit offset
        let frame_addr = pt_entry.addr().as_u64();
        Some(PhysAddr::new(frame_addr + offset))
    }
}

impl fmt::Debug for ActivePageTables {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActivePageTables")
            .field("pml4_addr", &self.pml4.as_ptr() as *const _)
            .finish()
    }
}

/// Manager for page table operations.
///
/// This type provides safe methods to create, modify, and remove page mappings.
/// It requires a `FrameAllocator` to allocate new page tables as needed.
///
/// # Type Parameters
///
/// - `A`: A frame allocator implementing `FrameAllocator<Size4KiB>`
///
/// # Capability Model
///
/// Holding a `PageTableManager` grants the capability to:
/// - Create new virtual memory mappings
/// - Remove existing mappings
/// - Query and modify page permissions
///
/// This capability should be restricted to trusted kernel code.
/// User processes should not have direct access to page table manipulation.
pub struct PageTableManager<A>
where
    A: FrameAllocator<Size4KiB>,
{
    /// Reference to the PML4 table we're managing.
    ///
    /// This is a static mutable reference because page tables are
    /// long-lived structures that persist for the lifetime of the address space.
    pml4: &'static mut PageTable,

    /// Allocator for creating new page tables.
    ///
    /// When walking the page table hierarchy, intermediate tables
    /// (PDPT, PD, PT) may need to be allocated. This allocator
    /// provides physical frames for those tables.
    frame_allocator: A,
}

impl<A> PageTableManager<A>
where
    A: FrameAllocator<Size4KiB>,
{
    /// Create a new page table manager.
    ///
    /// # Arguments
    ///
    /// - `pml4`: Mutable reference to the PML4 table
    /// - `frame_allocator`: Allocator for creating new page tables
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - `pml4` points to a valid, properly aligned page table
    /// - `pml4` is not being concurrently modified by another thread
    /// - The frame allocator is valid and has available frames
    pub fn new(pml4: &'static mut PageTable, frame_allocator: A) -> Self {
        Self {
            pml4,
            frame_allocator,
        }
    }

    /// Map a virtual page to a physical frame with the given flags.
    ///
    /// This is the core function for creating virtual memory mappings.
    /// It walks (or creates) the page table hierarchy and installs
    /// the final mapping in the leaf page table.
    ///
    /// # Arguments
    ///
    /// - `page`: The virtual page to map (4KB-aligned)
    /// - `frame`: The physical frame to map to (4KB-aligned)
    /// - `flags`: Permission and attribute flags for the mapping
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err(&str)` with error message on failure
    ///
    /// # Errors
    ///
    /// - `"Page is already mapped"` - The virtual page already has a mapping
    /// - `"Out of memory"` - Could not allocate a new page table
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The virtual address is not already mapped (or handle overwriting)
    /// - The physical frame is not already mapped elsewhere (unless intended for aliasing)
    /// - Proper synchronization if multiple CPUs access the same mapping
    /// - Flags are appropriate for the use case (e.g., don't mark MMIO as executable)
    ///
    /// # Capability Enforcement
    ///
    /// In a capability-based system, this function should only be callable
    /// by code that holds the appropriate capability to modify the address space.
    /// Typically, this means only the kernel (or a trusted memory manager).
    pub fn map_page(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame<Size4KiB>,
        flags: MapFlags,
    ) -> Result<(), &'static str> {
        let pml4 = self.pml4;

        // Extract indices for each level of the page table hierarchy
        let pml4_idx = page.p4_index();
        let pdpt_idx = page.p3_index();
        let pd_idx = page.p2_index();
        let pt_idx = page.p1_index();

        // Level 4: Ensure PDPT exists (allocate if needed)
        let pdpt = self.get_or_create_table(pml4, pml4_idx)?;

        // Level 3: Ensure PD exists (allocate if needed)
        let pd = self.get_or_create_table(pdpt, pdpt_idx)?;

        // Level 2: Ensure PT exists (allocate if needed)
        let pt = self.get_or_create_table(pd, pd_idx)?;

        // Level 1: Create the actual mapping in the page table
        let entry = &mut pt[pt_idx as usize];

        // Check if page is already mapped
        if entry.flags().contains(PageTableFlags::PRESENT) {
            return Err("Page is already mapped");
        }

        // Install the mapping
        entry.set_addr(frame.start_address(), PageTableFlags::from(flags));

        Ok(())
    }

    /// Get or create a page table at the given index.
    ///
    /// This is a helper function that checks if the next level page table
    /// exists, and allocates a new one if necessary.
    ///
    /// # Arguments
    ///
    /// - `table`: Current level page table (PML4, PDPT, or PD)
    /// - `index`: Index into the table for the next level
    ///
    /// # Returns
    ///
    /// - `Ok(&mut PageTable)` - Reference to the next level table
    /// - `Err(&str)` - Error message if allocation fails
    ///
    /// # Side Effects
    ///
    /// If a new table is allocated, it is:
    /// 1. Zero-initialized (all entries cleared)
    /// 2. Linked into the parent table with PRESENT | WRITABLE | USER_ACCESSIBLE
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

            // Convert to kernel virtual address for access
            let table_addr = phys_to_virt(frame.start_address());
            let new_table = unsafe { &mut *(table_addr.as_mut_ptr::<PageTable>()) };

            // Zero out the new page table (critical for security!)
            // Uninitialized page tables could leak information or cause faults
            *new_table = PageTable::new();

            // Link the new table into the hierarchy
            // We use PRESENT | WRITABLE | USER_ACCESSIBLE for all internal tables
            // The actual user/kernel distinction is made at the leaf level
            entry.set_addr(
                frame.start_address(),
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            );

            return Ok(new_table);
        }

        // Table already exists, get a reference to it
        let table_addr = PhysAddr::new(entry.addr().as_u64());
        let table_ptr = phys_to_virt(table_addr).as_mut_ptr::<PageTable>();
        Ok(unsafe { &mut *table_ptr })
    }

    /// Unmap a virtual page.
    ///
    /// This removes the mapping for a virtual page and returns the
    /// physical frame that was mapped. The caller is responsible for
    /// deallocating the frame if it's no longer needed.
    ///
    /// # Arguments
    ///
    /// - `page`: The virtual page to unmap
    ///
    /// # Returns
    ///
    /// - `Ok(PhysFrame)` - The physical frame that was unmapped
    /// - `Err(&str)` - Error message if the page was not mapped
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - No code is executing from this page (would cause #PF)
    /// - No references to this page exist (dangling pointers)
    /// - The physical frame is properly handled after unmapping
    ///
    /// # Side Effects
    ///
    /// - The TLB entry for this page is flushed
    /// - The page table entry is cleared
    /// - Note: Intermediate page tables are NOT freed (could be optimized)
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

        // Extract the physical frame before clearing the entry
        let frame = PhysFrame::from_start_address(entry.addr())
            .map_err(|_| "Invalid frame address")?;

        // Clear the entry (remove the mapping)
        entry.set_unused();

        // Flush the TLB entry for this page
        // This is critical - without this, the CPU might still use the old mapping
        unsafe {
            x86_64::instructions::tlb::flush(page.start_address());
        }

        Ok(frame)
    }

    /// Update the flags for an existing mapping.
    ///
    /// This allows changing permissions without modifying the physical frame.
    /// Useful for implementing copy-on-write, making pages read-only, etc.
    ///
    /// # Arguments
    ///
    /// - `page`: The virtual page to update
    /// - `flags`: New flags for the mapping
    ///
    /// # Returns
    ///
    /// - `Ok(())` on success
    /// - `Err(&str)` if the page is not mapped
    ///
    /// # Use Cases
    ///
    /// - Copy-on-write: Change WRITABLE from 1 to 0 on fork
    /// - Memory protection: mprotect()-style permission changes
    /// - Debugging: Make pages read-only to detect writes
    pub fn update_flags(&mut self, page: Page<Size4KiB>, flags: MapFlags) -> Result<(), &'static str> {
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

        // Preserve the physical address, update only the flags
        let addr = entry.addr();
        entry.set_addr(addr, PageTableFlags::from(flags));

        // Flush TLB to ensure new flags take effect immediately
        unsafe {
            x86_64::instructions::tlb::flush(page.start_address());
        }

        Ok(())
    }

    /// Get a reference to a page table at the given index.
    ///
    /// Helper function for traversing the page table hierarchy.
    /// Returns an error if the table doesn't exist (not present).
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

/// Identity map the kernel region.
///
/// This creates mappings for the kernel code and data, placing them
/// at their designated virtual addresses (KERNEL_OFFSET + physical address).
///
/// # Arguments
///
/// - `manager`: Page table manager to create mappings
/// - `kernel_start`: Physical start address of the kernel
/// - `kernel_end`: Physical end address of the kernel
/// - `code_end`: Physical end of kernel code section (for proper permissions)
///
/// # Returns
///
/// - `Ok(())` on success
/// - `Err(&str)` on mapping failure
///
/// # Implementation Notes
///
/// A production kernel would parse the ELF program headers to determine
/// exact section boundaries (.text, .rodata, .data, .bss) and map each
/// with appropriate permissions. This simplified version maps everything
/// as kernel data for demonstration purposes.
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
        // Calculate virtual address: physical + KERNEL_OFFSET
        let virt_addr = KERNEL_OFFSET + current_frame.start_address().as_u64();
        let page = Page::containing_address(virt_addr);

        // For simplicity, map everything as kernel data
        // A real implementation would use different flags for:
        // - .text section: KERNEL_CODE (readable, executable, not writable)
        // - .rodata section: read-only kernel data
        // - .data/.bss sections: KERNEL_DATA (readable, writable, not executable)
        let flags = MapFlags::KERNEL_DATA;

        manager.map_page(page, current_frame, flags)?;

        current_frame += 1;
    }

    Ok(())
}

/// Create an identity mapping for physical memory.
///
/// This maps physical addresses 0..MAX_PHYSICAL to virtual addresses
/// KERNEL_OFFSET..(KERNEL_OFFSET + MAX_PHYSICAL).
///
/// This is useful for:
/// - Accessing hardware devices (MMIO)
/// - Implementing a generic physical memory allocator
/// - Debugging and introspection
///
/// # Arguments
///
/// - `manager`: Page table manager
/// - `max_physical`: Maximum physical address to map
///
/// # Warning
///
/// This can consume significant memory for page tables on systems
/// with large amounts of RAM. Only map what you actually need.
pub fn identity_map_physical<A>(
    manager: &mut PageTableManager<A>,
    max_physical: PhysAddr,
) -> Result<(), &'static str>
where
    A: FrameAllocator<Size4KiB>,
{
    let end_frame = PhysFrame::containing_address(max_physical);
    let mut current_frame = PhysFrame::from_start_address(PhysAddr::new(0)).unwrap();

    while current_frame <= end_frame {
        let virt_addr = KERNEL_OFFSET + current_frame.start_address().as_u64();
        let page = Page::containing_address(virt_addr);

        // Identity map: virtual address = physical address + KERNEL_OFFSET
        // Flags: kernel-only, writable, cached
        let flags = MapFlags::KERNEL_DATA;

        manager.map_page(page, current_frame, flags)?;

        current_frame += 1;
    }

    Ok(())
}

/// Initialize the paging subsystem.
///
/// This function performs any necessary initialization for the paging system.
/// Currently, it just logs configuration information.
///
/// # Called During
///
/// Early kernel initialization, after console output is available
/// but before any memory allocations.
pub fn init() {
    log::info!("Paging subsystem initialized");
    log::info!("Kernel offset: {:#018x}", KERNEL_OFFSET.as_u64());
    log::info!("Virtual address space: 48-bit (256 TB total)");
    log::info!("User space: 0x0000000000000000 - 0x00007FFFFFFFFFFF (128 TB)");
    log::info!("Kernel space: 0xFFFF800000000000 - 0xFFFFFFFFFFFFFFFF (128 TB)");
    log::info!("Page size: 4 KB (standard), 2 MB / 1 GB (huge pages supported)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_offset_conversion() {
        let phys = PhysAddr::new(0x100000); // 1 MB
        let virt = phys_to_virt(phys);
        assert_eq!(virt.as_u64(), KERNEL_OFFSET.as_u64() + 0x100000);

        let recovered_phys = virt_to_phys(virt);
        assert_eq!(recovered_phys, phys);
    }

    #[test]
    fn test_map_flags_combinations() {
        // Test that flag combinations work correctly
        let user_rw = MapFlags::USER_DATA;
        assert!(user_rw.contains(MapFlags::PRESENT));
        assert!(user_rw.contains(MapFlags::WRITABLE));
        assert!(user_rw.contains(MapFlags::USER_ACCESSIBLE));
        assert!(user_rw.contains(MapFlags::NO_EXECUTE));

        let kernel_ro = MapFlags::KERNEL_CODE;
        assert!(kernel_ro.contains(MapFlags::PRESENT));
        assert!(!kernel_ro.contains(MapFlags::WRITABLE));
        assert!(!kernel_ro.contains(MapFlags::USER_ACCESSIBLE));
    }
}
