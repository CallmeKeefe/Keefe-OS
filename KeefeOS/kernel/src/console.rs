//! Console Output for KeefeOS
//!
//! This module provides serial diagnostics and an optional, experimental GOP framebuffer writer.
//!
//! VGA text memory at 0xB8000 is not assumed to be mapped or usable. The legacy
//! VGA writer is available only with the opt-in `legacy-vga-console` feature
//! and must not be enabled without a target-specific mapping guarantee.
//!
//! Early boot output is serial-first. GOP framebuffer output is enabled only
//! after the validated BootInfo handoff proves the whole framebuffer range safe.

use core::fmt::{self, Write};
use core::ptr::NonNull;
use crate::bootinfo::{BootInfo, FRAMEBUFFER_FORMAT_BGR, FRAMEBUFFER_FORMAT_RGB};
use spin::Mutex;
use x86_64::instructions::port::Port;

/// Standard VGA text mode colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[cfg(feature = "legacy-vga-console")]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

/// A color code for VGA text mode
/// Combines foreground and background colors into a single byte
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
#[cfg(feature = "legacy-vga-console")]
pub struct ColorCode(u8);

#[cfg(feature = "legacy-vga-console")]
impl ColorCode {
    /// Create a new color code with the specified foreground and background colors
    pub const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A single character cell in the VGA text buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
#[cfg(feature = "legacy-vga-console")]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

#[cfg(feature = "legacy-vga-console")]
impl ScreenChar {
    /// Create a new screen character
    pub const fn new(character: u8, color_code: ColorCode) -> Self {
        ScreenChar {
            ascii_character: character,
            color_code,
        }
    }
}

/// VGA text buffer dimensions
#[cfg(feature = "legacy-vga-console")]
const BUFFER_HEIGHT: usize = 25;
#[cfg(feature = "legacy-vga-console")]
const BUFFER_WIDTH: usize = 80;

/// VGA buffer memory address (standard location)
#[cfg(feature = "legacy-vga-console")]
const VGA_BUFFER_ADDRESS: usize = 0xB8000;

/// The VGA text buffer structure
#[repr(transparent)]
#[cfg(feature = "legacy-vga-console")]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// Writer to the VGA text buffer
#[cfg(feature = "legacy-vga-console")]
pub struct VgaWriter {
    column_position: usize,
    color_code: ColorCode,
    buffer: NonNull<Buffer>,
}

// SAFETY: All access to the VGA buffer is serialized through the global
// `VGA_WRITER` mutex, so sending the writer between contexts is safe.
#[cfg(feature = "legacy-vga-console")]
unsafe impl Send for VgaWriter {}

#[cfg(feature = "legacy-vga-console")]
impl VgaWriter {
    /// Create a new VGA writer with default colors
    pub const fn new() -> Self {
        VgaWriter {
            column_position: 0,
            color_code: ColorCode::new(Color::LightGray, Color::Black),
            // SAFETY: 0xB8000 is a valid, always-present MMIO address for the
            // VGA text buffer on x86 platforms. It is stored as a `NonNull`
            // raw pointer and only dereferenced while the global lock is held.
            buffer: unsafe { NonNull::new_unchecked(VGA_BUFFER_ADDRESS as *mut Buffer) },
        }
    }

    /// Set the text color
    pub fn set_color(&mut self, color_code: ColorCode) {
        self.color_code = color_code;
    }

    /// Write a byte to the VGA buffer (handles newlines and wrapping)
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                // SAFETY: `self.buffer` points at the valid VGA MMIO region.
                unsafe {
                    core::ptr::write_volatile(
                        self.buffer.as_ptr().cast::<ScreenChar>().add(row * BUFFER_WIDTH + col),
                        ScreenChar::new(byte, self.color_code),
                    );
                }
                self.column_position += 1;
            }
        }
    }

    /// Write a string slice to the VGA buffer
    pub fn write_message(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII or newline
                0x20..=0x7E | b'\n' => self.write_byte(byte),
                // Non-printable: show as replacement character
                _ => self.write_byte(0xFE),
            }
        }
    }

    /// Scroll the buffer up by one line and clear the last line
    fn new_line(&mut self) {
        // Shift all lines up by one
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                // SAFETY: valid MMIO region, see `VgaWriter::new`.
                let character = unsafe {
                    core::ptr::read_volatile(
                        self.buffer.as_ptr().cast::<ScreenChar>().add(row * BUFFER_WIDTH + col),
                    )
                };
                unsafe {
                    core::ptr::write_volatile(
                        self.buffer.as_ptr().cast::<ScreenChar>().add((row - 1) * BUFFER_WIDTH + col),
                        character,
                    );
                }
            }
        }

        // Clear the last line
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    /// Clear a specific row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar::new(b' ', self.color_code);
        for col in 0..BUFFER_WIDTH {
            // SAFETY: valid MMIO region, see `VgaWriter::new`.
            unsafe {
                core::ptr::write_volatile(
                    self.buffer.as_ptr().cast::<ScreenChar>().add(row * BUFFER_WIDTH + col),
                    blank,
                );
            }
        }
    }

    /// Clear the entire screen
    pub fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.column_position = 0;
    }
}

/// Implement fmt::Write for VgaWriter to enable format! macro support
#[cfg(feature = "legacy-vga-console")]
impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_message(s);
        Ok(())
    }
}

/// Global VGA writer instance
#[cfg(feature = "legacy-vga-console")]
pub static VGA_WRITER: Mutex<VgaWriter> = Mutex::new(VgaWriter::new());

/// Base I/O port of COM1
const COM1: u16 = 0x3F8;

/// Serial port writer for debugging output
/// Uses COM1 (I/O port 0x3F8) following the 16550 UART programming model
pub struct SerialWriter {
    data: Port<u8>,
    divisor_low: Port<u8>,
    divisor_high: Port<u8>,
    interrupt_enable: Port<u8>,
    fifo_control: Port<u8>,
    line_control: Port<u8>,
    modem_control: Port<u8>,
    line_status: Port<u8>,
}

impl SerialWriter {
    /// Create a new serial writer for COM1
    pub const fn new() -> Self {
        SerialWriter {
            data: Port::new(COM1),
            divisor_low: Port::new(COM1 + 0),
            divisor_high: Port::new(COM1 + 1),
            interrupt_enable: Port::new(COM1 + 1),
            fifo_control: Port::new(COM1 + 2),
            line_control: Port::new(COM1 + 3),
            modem_control: Port::new(COM1 + 4),
            line_status: Port::new(COM1 + 5),
        }
    }

    /// Initialize the serial port
    /// Configures: 9600 baud, 8 data bits, no parity, 1 stop bit
    pub fn init(&mut self) {
        unsafe {
            // Disable interrupts
            self.interrupt_enable.write(0x00);

            // Enable DLAB (set baud rate divisor)
            self.line_control.write(0x80);

            // Set divisor for 9600 baud (115200 / 9600 = 12)
            self.divisor_low.write(0x0C); // Low byte
            self.divisor_high.write(0x00); // High byte

            // 8 bits, no parity, one stop bit (clear DLAB)
            self.line_control.write(0x03);

            // Enable FIFO, clear them, with 14-byte threshold
            self.fifo_control.write(0xC7);

            // IRQs enabled, RTS/DSR set
            self.modem_control.write(0x0B);
        }
    }

    /// Check if the transmit holding buffer is empty
    #[inline]
    fn is_transmit_empty(&mut self) -> bool {
        // SAFETY: reading the line status register of the COM1 UART.
        unsafe { (self.line_status.read() & 0x20) != 0 }
    }

    /// Write a byte to the serial port
    pub fn write_byte(&mut self, byte: u8) {
        // Firmware/platforms may not expose a functioning legacy COM1 UART.
        // Bound the wait so diagnostics cannot hang the kernel indefinitely.
        let mut ready = false;
        for _ in 0..1_000_000 {
            if self.is_transmit_empty() {
                ready = true;
                break;
            }
            core::hint::spin_loop();
        }
        if !ready {
            return;
        }
        // SAFETY: writing to the COM1 data register.
        unsafe {
            self.data.write(byte);
        }
    }

    /// Write a string to the serial port
    pub fn write_message(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }
}

/// Implement fmt::Write for SerialWriter to enable format! macro support
impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_message(s);
        Ok(())
    }
}

/// Global serial writer instance
pub static SERIAL_WRITER: Mutex<Option<SerialWriter>> = Mutex::new(None);

/// Minimal 32-bit GOP framebuffer text renderer.
///
/// Direct writes are enabled only when BootInfo contains validated RGB/BGR
/// metadata and the frame buffer lies within a reserved firmware memory range.
/// The firmware's existing mapping is retained; this does not establish a
/// general virtual-memory mapping for arbitrary devices.
struct FramebufferWriter {
    base: NonNull<u8>,
    size: usize,
    width: usize,
    height: usize,
    stride: usize,
    pixel_format: u32,
    cursor_x: usize,
    cursor_y: usize,
}

unsafe impl Send for FramebufferWriter {}

impl FramebufferWriter {
    fn from_boot_info(info: &BootInfo) -> Option<Self> {
        if info.fb_addr == 0
            || info.fb_size == 0
            || (info.fb_pixel_format != FRAMEBUFFER_FORMAT_RGB
                && info.fb_pixel_format != FRAMEBUFFER_FORMAT_BGR)
        {
            return None;
        }

        let base = NonNull::new(info.fb_addr as *mut u8)?;
        let size = usize::try_from(info.fb_size).ok()?;
        let width = info.fb_horiz_res as usize;
        let height = info.fb_vert_res as usize;
        let stride = info.fb_pixels_per_scan_line as usize;
        if width == 0 || height == 0 || stride < width {
            return None;
        }
        let required = stride.checked_mul(height)?.checked_mul(4)?;
        if required > size {
            return None;
        }

        Some(Self {
            base,
            size,
            width,
            height,
            stride,
            pixel_format: info.fb_pixel_format,
            cursor_x: 16,
            cursor_y: 88,
        })
    }

    fn pixel(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let Some(offset) = y
            .checked_mul(self.stride)
            .and_then(|row| row.checked_add(x))
            .and_then(|pixel| pixel.checked_mul(4))
        else {
            return;
        };
        let Some(end) = offset.checked_add(4) else {
            return;
        };
        if end > self.size {
            return;
        }
        let Some(bytes) = encode_pixel(self.pixel_format, rgb) else {
            return;
        };

        // SAFETY: BootInfo validates dimensions, stride, size, address overflow,
        // pixel format, and reserved memory-map coverage before this writer is
        // created. The offset is checked against the reported buffer size, and
        // writes are volatile because the target is a device framebuffer.
        unsafe {
            for (index, value) in bytes.iter().enumerate() {
                core::ptr::write_volatile(self.base.as_ptr().add(offset + index), *value);
            }
        }
    }

    fn fill_rect(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: [u8; 3],
    ) {
        let right = x.saturating_add(width).min(self.width);
        let bottom = y.saturating_add(height).min(self.height);
        for py in y.min(self.height)..bottom {
            for px in x.min(self.width)..right {
                self.pixel(px, py, color);
            }
        }
    }

    fn clear_screen(&mut self) {
        self.fill_rect(0, 0, self.width, self.height, [8, 14, 28]);
        self.fill_rect(0, 0, self.width, 4, [57, 224, 198]);
    }

    fn draw_char_scaled(
        &mut self,
        ch: u8,
        x: usize,
        y: usize,
        scale: usize,
        color: [u8; 3],
    ) {
        let columns = glyph(ch);
        for (column, pattern) in columns.iter().enumerate() {
            for row in 0usize..7 {
                if *pattern & (1u8 << row) == 0 {
                    continue;
                }
                let px = x.saturating_add(column.saturating_mul(scale));
                let py = y.saturating_add(row.saturating_mul(scale));
                self.fill_rect(px, py, scale, scale, color);
            }
        }
    }

    fn draw_text_scaled(
        &mut self,
        text: &str,
        x: usize,
        y: usize,
        scale: usize,
        color: [u8; 3],
    ) {
        let mut cursor = x;
        let advance = 6usize.saturating_mul(scale);
        for byte in text.bytes() {
            self.draw_char_scaled(byte, cursor, y, scale, color);
            cursor = cursor.saturating_add(advance);
        }
    }

    fn newline(&mut self) {
        self.cursor_x = 16;
        self.cursor_y = self.cursor_y.saturating_add(10);
        if self.cursor_y.saturating_add(8) >= self.height {
            self.clear_screen();
            self.draw_text_scaled("KEEFEOS", 16, 18, 2, [235, 244, 255]);
            self.cursor_y = 68;
        }
    }

    fn write_byte(&mut self, byte: u8) {
        if byte == b'\n' {
            self.newline();
            return;
        }
        if self.cursor_x.saturating_add(6) >= self.width {
            self.newline();
        }
        self.draw_char_scaled(byte, self.cursor_x, self.cursor_y, 1, [218, 232, 248]);
        self.cursor_x = self.cursor_x.saturating_add(6);
    }
}

impl fmt::Write for FramebufferWriter {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

static FRAMEBUFFER_WRITER: Mutex<Option<FramebufferWriter>> = Mutex::new(None);

fn encode_pixel(pixel_format: u32, rgb: [u8; 3]) -> Option<[u8; 4]> {
    match pixel_format {
        FRAMEBUFFER_FORMAT_RGB => Some([rgb[0], rgb[1], rgb[2], 0]),
        FRAMEBUFFER_FORMAT_BGR => Some([rgb[2], rgb[1], rgb[0], 0]),
        _ => None,
    }
}

fn glyph(byte: u8) -> [u8; 5] {
    match byte.to_ascii_uppercase() {
        b'A' => [0x7E, 0x11, 0x11, 0x11, 0x7E],
        b'B' => [0x7F, 0x49, 0x49, 0x49, 0x36],
        b'C' => [0x3E, 0x41, 0x41, 0x41, 0x22],
        b'D' => [0x7F, 0x41, 0x41, 0x22, 0x1C],
        b'E' => [0x7F, 0x49, 0x49, 0x49, 0x41],
        b'F' => [0x7F, 0x09, 0x09, 0x09, 0x01],
        b'G' => [0x3E, 0x41, 0x49, 0x49, 0x7A],
        b'H' => [0x7F, 0x08, 0x08, 0x08, 0x7F],
        b'I' => [0x00, 0x41, 0x7F, 0x41, 0x00],
        b'J' => [0x20, 0x40, 0x41, 0x3F, 0x01],
        b'K' => [0x7F, 0x08, 0x14, 0x22, 0x41],
        b'L' => [0x7F, 0x40, 0x40, 0x40, 0x40],
        b'M' => [0x7F, 0x02, 0x0C, 0x02, 0x7F],
        b'N' => [0x7F, 0x04, 0x08, 0x10, 0x7F],
        b'O' => [0x3E, 0x41, 0x41, 0x41, 0x3E],
        b'P' => [0x7F, 0x09, 0x09, 0x09, 0x06],
        b'Q' => [0x3E, 0x41, 0x51, 0x21, 0x5E],
        b'R' => [0x7F, 0x09, 0x19, 0x29, 0x46],
        b'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        b'T' => [0x01, 0x01, 0x7F, 0x01, 0x01],
        b'U' => [0x3F, 0x40, 0x40, 0x40, 0x3F],
        b'V' => [0x1F, 0x20, 0x40, 0x20, 0x1F],
        b'W' => [0x3F, 0x40, 0x38, 0x40, 0x3F],
        b'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        b'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        b'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        b'0' => [0x3E, 0x51, 0x49, 0x45, 0x3E],
        b'1' => [0x00, 0x42, 0x7F, 0x40, 0x00],
        b'2' => [0x62, 0x51, 0x49, 0x49, 0x46],
        b'3' => [0x22, 0x41, 0x49, 0x49, 0x36],
        b'4' => [0x18, 0x14, 0x12, 0x7F, 0x10],
        b'5' => [0x2F, 0x49, 0x49, 0x49, 0x31],
        b'6' => [0x3E, 0x49, 0x49, 0x49, 0x32],
        b'7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        b'8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        b'9' => [0x26, 0x49, 0x49, 0x49, 0x3E],
        b':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        b'.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        b'-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        b'/' => [0x20, 0x10, 0x08, 0x04, 0x02],
        b'[' => [0x00, 0x7F, 0x41, 0x41, 0x00],
        b']' => [0x00, 0x41, 0x41, 0x7F, 0x00],
        b'(' => [0x00, 0x1C, 0x22, 0x41, 0x00],
        b')' => [0x00, 0x41, 0x22, 0x1C, 0x00],
        b'_' => [0x40, 0x40, 0x40, 0x40, 0x40],
        b' ' => [0; 5],
        _ => [0x7F, 0x41, 0x5D, 0x41, 0x7F],
    }
}

/// Initialize a checked GOP framebuffer writer after BootInfo validation.
///
/// Returns false when the loader did not expose a supported, reserved RGB/BGR
/// framebuffer. Serial output remains available regardless.
pub fn init_framebuffer(info: &BootInfo) -> bool {
    let Some(mut writer) = FramebufferWriter::from_boot_info(info) else {
        return false;
    };
    writer.clear_screen();
    writer.draw_text_scaled("KEEFEOS", 16, 18, 3, [235, 244, 255]);
    writer.draw_text_scaled("EARLY BOOT VERIFIED", 18, 54, 1, [57, 224, 198]);
    writer.cursor_x = 16;
    writer.cursor_y = 82;
    *FRAMEBUFFER_WRITER.lock() = Some(writer);
    true
}

#[cfg(test)]
mod framebuffer_tests {
    use super::*;

    #[test]
    fn rgb_and_bgr_pixels_have_correct_byte_order() {
        assert_eq!(encode_pixel(FRAMEBUFFER_FORMAT_RGB, [1, 2, 3]), Some([1, 2, 3, 0]));
        assert_eq!(encode_pixel(FRAMEBUFFER_FORMAT_BGR, [1, 2, 3]), Some([3, 2, 1, 0]));
        assert_eq!(encode_pixel(2, [1, 2, 3]), None);
    }

    #[test]
    fn glyphs_are_fixed_width() {
        assert_eq!(glyph(b'A').len(), 5);
        assert_ne!(glyph(b'K'), [0; 5]);
    }
}

/// Initialize the console subsystem
pub fn init() {
    // Initialize serial port
    {
        let mut serial = SERIAL_WRITER.lock();
        let mut writer = SerialWriter::new();
        writer.init();
        *serial = Some(writer);
    }

    // Do not touch VGA text memory: UEFI does not promise legacy VGA mode.
    log::info!("Console initialized (COM1 serial only; GOP is not wired yet)");
}

/// Write a byte to both VGA and serial outputs
pub fn write_byte(byte: u8) {
    // Serial always remains available as the fallback diagnostic channel.
    {
        let mut serial = SERIAL_WRITER.lock();
        if let Some(ref mut writer) = *serial {
            writer.write_byte(byte);
        }
    }
    if let Some(ref mut writer) = *FRAMEBUFFER_WRITER.lock() {
        writer.write_byte(byte);
    }
}

/// Write a string to both outputs
pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Print formatted output (used by the print! macro)
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // Serial is always the diagnostic fallback.
    {
        let mut serial = SERIAL_WRITER.lock();
        if let Some(ref mut writer) = *serial {
            // Console output is best-effort. Never panic while reporting another
            // failure, which could recurse through the panic handler.
            let _ = writer.write_fmt(args);
        }
    }
    if let Some(ref mut writer) = *FRAMEBUFFER_WRITER.lock() {
        // Keep formatting/output failures from recursively panicking.
        let _ = writer.write_fmt(args);
    }
}

/// Macro for printing to the console
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::console::_print(format_args!($($arg)*))
    };
}

/// Macro for printing with a newline
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
