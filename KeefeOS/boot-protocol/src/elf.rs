//! Bounds-checked ELF64 parser for the x86-64 identity-mapped kernel image.
//!
//! The parser accepts a deliberately narrow format: little-endian ELF64,
//! ET_EXEC, EM_X86_64, physical and virtual load addresses equal, and all load
//! segments within the linker script's reserved 32–64 MiB identity-mapped window. It parses bytes only;
//! the caller remains responsible for reserving destination pages and copying
//! segments before transferring control.


use core::convert::TryFrom;

/// ELF errors are explicit so malformed input cannot be mistaken for a valid image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// File is too short to contain the ELF header.
    TruncatedHeader,
    /// ELF magic, class, byte order, or version is unsupported.
    InvalidIdentification,
    /// ELF type or target machine is unsupported.
    UnsupportedType,
    /// Header sizes or program-header metadata are invalid.
    InvalidHeader,
    /// Program-header table exceeds the input buffer.
    ProgramHeaderOutOfBounds,
    /// No loadable kernel segments were present.
    NoLoadSegments,
    /// Segment metadata is invalid or self-inconsistent.
    InvalidSegment,
    /// Load segments overlap in physical memory.
    OverlappingSegments,
    /// Load segment points outside the permitted low identity-mapped range.
    AddressOutOfRange,
    /// Entry point is not within an executable load segment.
    InvalidEntryPoint,
    /// A file offset/address cannot be represented by the current target.
    IntegerOverflow,
}

const ELF_HEADER_SIZE: usize = 64;
const ELF_PROGRAM_HEADER_SIZE: usize = 56;
const PT_LOAD: u32 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const PF_X: u32 = 1;
const PAGE_SIZE: u64 = 4096;
const MIN_KERNEL_ADDRESS: u64 = 0x0200_0000;
const MAX_KERNEL_ADDRESS: u64 = 0x0400_0000; // exclusive 64 MiB upper bound for the 32 MiB load window
const MAX_PROGRAM_HEADERS: usize = 1024;

/// Validated ELF64 load segment metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadSegment {
    file_offset: u64,
    virtual_address: u64,
    physical_address: u64,
    file_size: u64,
    memory_size: u64,
    flags: u32,
    alignment: u64,
}

impl LoadSegment {
    /// File offset where this segment's initialized bytes begin.
    pub const fn file_offset(&self) -> u64 { self.file_offset }
    /// Virtual address (required to equal physical address in this boot protocol).
    pub const fn virtual_address(&self) -> u64 { self.virtual_address }
    /// Physical destination address.
    pub const fn physical_address(&self) -> u64 { self.physical_address }
    /// Number of initialized bytes in the ELF file.
    pub const fn file_size(&self) -> u64 { self.file_size }
    /// Total in-memory segment size, including zero-initialized BSS.
    pub const fn memory_size(&self) -> u64 { self.memory_size }
    /// ELF segment flags.
    pub const fn flags(&self) -> u32 { self.flags }
    /// ELF segment alignment.
    pub const fn alignment(&self) -> u64 { self.alignment }
    /// Exclusive physical end address.
    pub fn physical_end(&self) -> Option<u64> {
        self.physical_address.checked_add(self.memory_size)
    }
    /// Whether this segment contains executable bytes.
    pub const fn is_executable(&self) -> bool { (self.flags & PF_X) != 0 }
}

/// Parsed, bounds-checked ELF64 image metadata.
#[derive(Debug, Clone, Copy)]
pub struct ElfImage<'a> {
    bytes: &'a [u8],
    entry: u64,
    program_header_offset: usize,
    program_header_size: usize,
    program_header_count: usize,
    load_start: u64,
    load_end: u64,
}

impl<'a> ElfImage<'a> {
    /// Parse and validate an x86-64 ET_EXEC ELF image for the fixed 32–64 MiB identity-mapped load window.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ElfError> {
        if bytes.len() < ELF_HEADER_SIZE {
            return Err(ElfError::TruncatedHeader);
        }
        if &bytes[0..4] != b"\x7fELF"
            || bytes[4] != 2 // ELFCLASS64
            || bytes[5] != 1 // ELFDATA2LSB
            || bytes[6] != 1 // EV_CURRENT
        {
            return Err(ElfError::InvalidIdentification);
        }
        if read_u16(bytes, 16)? != ET_EXEC || read_u16(bytes, 18)? != EM_X86_64
            || read_u32(bytes, 20)? != 1
        {
            return Err(ElfError::UnsupportedType);
        }
        if read_u16(bytes, 52)? as usize != ELF_HEADER_SIZE {
            return Err(ElfError::InvalidHeader);
        }

        let entry = read_u64(bytes, 24)?;
        let phoff = usize::try_from(read_u64(bytes, 32)?).map_err(|_| ElfError::IntegerOverflow)?;
        let phentsize = read_u16(bytes, 54)? as usize;
        let phnum = read_u16(bytes, 56)? as usize;
        if phentsize < ELF_PROGRAM_HEADER_SIZE || phentsize > 4096
            || phnum == 0 || phnum > MAX_PROGRAM_HEADERS
        {
            return Err(ElfError::InvalidHeader);
        }
        let table_bytes = phentsize.checked_mul(phnum).ok_or(ElfError::IntegerOverflow)?;
        let table_end = phoff.checked_add(table_bytes).ok_or(ElfError::IntegerOverflow)?;
        if phoff < ELF_HEADER_SIZE || table_end > bytes.len() {
            return Err(ElfError::ProgramHeaderOutOfBounds);
        }

        let mut image = Self {
            bytes, entry, program_header_offset: phoff,
            program_header_size: phentsize, program_header_count: phnum,
            load_start: u64::MAX, load_end: 0,
        };

        let mut load_count = 0usize;
        let mut entry_is_executable = false;
        for i in 0..phnum {
            if let Some(segment) = image.segment_at(i)? {
                load_count += 1;
                let end = segment.physical_end().ok_or(ElfError::IntegerOverflow)?;
                image.load_start = image.load_start.min(segment.physical_address);
                image.load_end = image.load_end.max(end);
                if segment.is_executable() && entry >= segment.physical_address && entry < end {
                    entry_is_executable = true;
                }

                for previous_index in 0..i {
                    if let Some(previous) = image.segment_at(previous_index)? {
                        let previous_end = previous.physical_end().ok_or(ElfError::IntegerOverflow)?;
                        if segment.physical_address < previous_end
                            && previous.physical_address < end
                        {
                            return Err(ElfError::OverlappingSegments);
                        }
                    }
                }
            }
        }

        if load_count == 0 {
            return Err(ElfError::NoLoadSegments);
        }
        if !entry_is_executable {
            return Err(ElfError::InvalidEntryPoint);
        }
        image.load_start &= !(PAGE_SIZE - 1);
        image.load_end = image.load_end.checked_add(PAGE_SIZE - 1)
            .ok_or(ElfError::IntegerOverflow)? & !(PAGE_SIZE - 1);
        if image.load_start < MIN_KERNEL_ADDRESS || image.load_end > MAX_KERNEL_ADDRESS
            || image.load_end <= image.load_start
        {
            return Err(ElfError::AddressOutOfRange);
        }
        Ok(image)
    }

    /// Kernel entry point from the validated ELF header.
    pub const fn entry(&self) -> u64 { self.entry }
    /// Page-aligned start of the single reserved kernel load span.
    pub const fn load_start(&self) -> u64 { self.load_start }
    /// Page-aligned exclusive end of the single reserved kernel load span.
    pub const fn load_end(&self) -> u64 { self.load_end }
    /// Number of program headers in the image.
    pub const fn program_header_count(&self) -> usize { self.program_header_count }

    /// Return a validated PT_LOAD segment at the given program-header index.
    /// Non-load segments return Ok(None).
    pub fn segment_at(&self, index: usize) -> Result<Option<LoadSegment>, ElfError> {
        if index >= self.program_header_count {
            return Ok(None);
        }
        let offset = self.program_header_offset
            .checked_add(index.checked_mul(self.program_header_size).ok_or(ElfError::IntegerOverflow)?)
            .ok_or(ElfError::IntegerOverflow)?;
        let end = offset.checked_add(ELF_PROGRAM_HEADER_SIZE).ok_or(ElfError::IntegerOverflow)?;
        if end > self.bytes.len() {
            return Err(ElfError::ProgramHeaderOutOfBounds);
        }
        let header = &self.bytes[offset..end];
        if read_u32(header, 0)? != PT_LOAD {
            return Ok(None);
        }

        let flags = read_u32(header, 4)?;
        let file_offset = read_u64(header, 8)?;
        let virtual_address = read_u64(header, 16)?;
        let physical_address = read_u64(header, 24)?;
        let file_size = read_u64(header, 32)?;
        let memory_size = read_u64(header, 40)?;
        let alignment = read_u64(header, 48)?;

        if memory_size == 0 || file_size > memory_size {
            return Err(ElfError::InvalidSegment);
        }
        let file_end = file_offset.checked_add(file_size).ok_or(ElfError::IntegerOverflow)?;
        if file_end > self.bytes.len() as u64 {
            return Err(ElfError::InvalidSegment);
        }
        let physical_end = physical_address.checked_add(memory_size).ok_or(ElfError::IntegerOverflow)?;
        if physical_address != virtual_address
            || physical_address < MIN_KERNEL_ADDRESS
            || physical_end > MAX_KERNEL_ADDRESS
            || physical_end <= physical_address
        {
            return Err(ElfError::AddressOutOfRange);
        }
        if alignment > 1 {
            if !alignment.is_power_of_two()
                || virtual_address % alignment != file_offset % alignment
            {
                return Err(ElfError::InvalidSegment);
            }
        }
        Ok(Some(LoadSegment {
            file_offset, virtual_address, physical_address,
            file_size, memory_size, flags, alignment,
        }))
    }

    /// Return initialized bytes for a segment previously returned by this image.
    pub fn segment_bytes(&self, segment: LoadSegment) -> Result<&'a [u8], ElfError> {
        let start = usize::try_from(segment.file_offset).map_err(|_| ElfError::IntegerOverflow)?;
        let size = usize::try_from(segment.file_size).map_err(|_| ElfError::IntegerOverflow)?;
        let end = start.checked_add(size).ok_or(ElfError::IntegerOverflow)?;
        self.bytes.get(start..end).ok_or(ElfError::InvalidSegment)
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ElfError> {
    let end = offset.checked_add(2).ok_or(ElfError::IntegerOverflow)?;
    let slice = bytes.get(offset..end).ok_or(ElfError::TruncatedHeader)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ElfError> {
    let end = offset.checked_add(4).ok_or(ElfError::IntegerOverflow)?;
    let slice = bytes.get(offset..end).ok_or(ElfError::TruncatedHeader)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ElfError> {
    let end = offset.checked_add(8).ok_or(ElfError::IntegerOverflow)?;
    let slice = bytes.get(offset..end).ok_or(ElfError::TruncatedHeader)?;
    Ok(u64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3],
        slice[4], slice[5], slice[6], slice[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    fn sample_elf() -> Vec<u8> {
        let mut bytes = vec![0u8; 121];
        bytes[0..4].copy_from_slice(b"\x7fELF");
        bytes[4] = 2;
        bytes[5] = 1;
        bytes[6] = 1;
        bytes[16..18].copy_from_slice(&ET_EXEC.to_le_bytes());
        bytes[18..20].copy_from_slice(&EM_X86_64.to_le_bytes());
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes[24..32].copy_from_slice(&0x0200_0000u64.to_le_bytes());
        bytes[32..40].copy_from_slice(&64u64.to_le_bytes());
        bytes[52..54].copy_from_slice(&(ELF_HEADER_SIZE as u16).to_le_bytes());
        bytes[54..56].copy_from_slice(&(ELF_PROGRAM_HEADER_SIZE as u16).to_le_bytes());
        bytes[56..58].copy_from_slice(&1u16.to_le_bytes());

        let ph = 64;
        bytes[ph..ph+4].copy_from_slice(&PT_LOAD.to_le_bytes());
        bytes[ph+4..ph+8].copy_from_slice(&5u32.to_le_bytes()); // R | X
        bytes[ph+8..ph+16].copy_from_slice(&120u64.to_le_bytes());
        bytes[ph+16..ph+24].copy_from_slice(&0x0200_0000u64.to_le_bytes());
        bytes[ph+24..ph+32].copy_from_slice(&0x0200_0000u64.to_le_bytes());
        bytes[ph+32..ph+40].copy_from_slice(&1u64.to_le_bytes());
        bytes[ph+40..ph+48].copy_from_slice(&0x1000u64.to_le_bytes());
        bytes[ph+48..ph+56].copy_from_slice(&1u64.to_le_bytes());
        bytes[120] = 0xC3;
        bytes
    }

    #[test]
    fn accepts_valid_fixed_window_executable() {
        let bytes = sample_elf();
        let image = ElfImage::parse(&bytes).unwrap();
        assert_eq!(image.entry(), 0x0200_0000);
        assert_eq!(image.load_start(), 0x0200_0000);
        assert_eq!(image.load_end(), 0x0200_1000);
        let segment = image.segment_at(0).unwrap().unwrap();
        assert_eq!(image.segment_bytes(segment).unwrap(), &[0xC3]);
    }

    #[test]
    fn rejects_truncated_header_and_bad_magic() {
        assert_eq!(ElfImage::parse(&[0u8; 20]).unwrap_err(), ElfError::TruncatedHeader);
        let mut bytes = sample_elf();
        bytes[0] = 0;
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::InvalidIdentification);
    }

    #[test]
    fn rejects_non_x86_64_images() {
        let mut bytes = sample_elf();
        bytes[18..20].copy_from_slice(&183u16.to_le_bytes()); // AArch64
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::UnsupportedType);
    }

    #[test]
    fn rejects_out_of_bounds_program_headers_and_segments() {
        let mut bytes = sample_elf();
        bytes[32..40].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::IntegerOverflow);

        let mut bytes = sample_elf();
        bytes[72..80].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::IntegerOverflow);
    }

    #[test]
    fn rejects_memsz_smaller_than_filesz() {
        let mut bytes = sample_elf();
        bytes[104..112].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::InvalidSegment);
    }

    #[test]
    fn rejects_entry_outside_executable_segment() {
        let mut bytes = sample_elf();
        bytes[68..72].copy_from_slice(&6u32.to_le_bytes()); // R | W; no execute permission
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::InvalidEntryPoint);
    }

    #[test]
    fn rejects_kernel_segments_outside_identity_window() {
        let mut bytes = sample_elf();
        bytes[24..32].copy_from_slice(&0xFFFF_8000_0000_0000u64.to_le_bytes());
        bytes[80..88].copy_from_slice(&0xFFFF_8000_0000_0000u64.to_le_bytes());
        bytes[88..96].copy_from_slice(&0xFFFF_8000_0000_0000u64.to_le_bytes());
        assert_eq!(ElfImage::parse(&bytes).unwrap_err(), ElfError::AddressOutOfRange);
    }
}
