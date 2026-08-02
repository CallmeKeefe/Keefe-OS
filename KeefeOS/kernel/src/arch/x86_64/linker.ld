/* KeefeOS Kernel Linker Script for x86_64
 * 
 * This linker script defines the memory layout of the kernel.
 * We use a higher-half kernel design where the kernel is mapped
 * at a high virtual address (0xFFFF800000000000) to separate it
 * from user-space memory (0x0000000000000000 - 0x00007FFFFFFFFFFF).
 * 
 * Memory Layout:
 * - 0x0000000000000000 - 0x00007FFFFFFFFFFF: User space (128 TB)
 * - 0x0000800000000000 - 0xFFFF7FFFFFFFFFFF: Unused (guard space)
 * - 0xFFFF800000000000 - 0xFFFFFFFFFFFFFFFF: Kernel space (128 TB)
 */

/* Define the entry point for the kernel */
ENTRY(kernel_start)

/* Define memory regions */
MEMORY
{
    /* The kernel will be loaded at 1MB (0x100000) by the bootloader */
    /* This avoids the legacy BIOS area (0xA0000 - 0xFFFFF) */
    KERNEL_RAM : ORIGIN = 0x00100000, LENGTH = 32M
}

/* Stack size for the kernel (per-CPU stacks are allocated separately) */
STACK_SIZE = 8K;

SECTIONS
{
    /* Start placing sections at the kernel load address */
    . = ORIGIN(KERNEL_RAM);

    /* Boot section - contains the entry point and early boot code */
    .boot :
    {
        KEEP(*(.boot))
        *(.boot.*)
    } > KERNEL_RAM

    /* Text section - executable code */
    .text : ALIGN(4K)
    {
        *(.text)
        *(.text.*)
    } > KERNEL_RAM

    /* Read-only data section - constants, strings */
    .rodata : ALIGN(4K)
    {
        *(.rodata)
        *(.rodata.*)
    } > KERNEL_RAM

    /* Data section - initialized global variables */
    .data : ALIGN(4K)
    {
        *(.data)
        *(.data.*)
    } > KERNEL_RAM

    /* BSS section - uninitialized global variables (zeroed at runtime) */
    .bss : ALIGN(4K)
    {
        _bss_start = .;
        *(.bss)
        *(.bss.*)
        *(COMMON)
        _bss_end = .;
    } > KERNEL_RAM

    /* Kernel stack section - for the initial CPU stack */
    .stack (NOLOAD) : ALIGN(16)
    {
        _stack_bottom = .;
        . = . + STACK_SIZE;
        _stack_top = .;
    } > KERNEL_RAM

    /* Discard unnecessary sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.note.GNU-stack)
    }

    /* Ensure the kernel doesn't exceed available memory */
    _kernel_end = .;
}

/* Define symbols for the kernel to use */
PROVIDE(_kernel_physical_start = ORIGIN(KERNEL_RAM));
PROVIDE(_kernel_physical_end = _kernel_end);
PROVIDE(_kernel_size = _kernel_end - ORIGIN(KERNEL_RAM));
