//! Console Output for KeefeOS
//!
//! This module provides basic console output functionality:
//! - VGA text buffer (16 colors, 80x25 resolution)
//! - Serial port output (for debugging and logging, COM1 16550 UART)
//!
//! Both outputs can be used simultaneously for redundancy.

use core::fmt::{self, Write};
use core::ptr::NonNull;
use spin::Mutex;
use x86_64::instructions::port::Port;

/// Standard VGA text mode colors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
pub struct ColorCode(u8);

impl ColorCode {
    /// Create a new color code with the specified foreground and background colors
    pub const fn new(foreground: Color, background: Color) -> Self {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A single character cell in the VGA text buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

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
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// VGA buffer memory address (standard location)
const VGA_BUFFER_ADDRESS: usize = 0xB8000;

/// The VGA text buffer structure
#[repr(transparent)]
struct Buffer {
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

/// Writer to the VGA text buffer
pub struct VgaWriter {
    column_position: usize,
    color_code: ColorCode,
    buffer: NonNull<Buffer>,
}

// SAFETY: All access to the VGA buffer is serialized through the global
// `VGA_WRITER` mutex, so sending the writer between contexts is safe.
unsafe impl Send for VgaWriter {}

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
impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_message(s);
        Ok(())
    }
}

/// Global VGA writer instance
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
        while !self.is_transmit_empty() {}
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

/// Initialize the console subsystem
pub fn init() {
    // Initialize serial port
    {
        let mut serial = SERIAL_WRITER.lock();
        let mut writer = SerialWriter::new();
        writer.init();
        *serial = Some(writer);
    }

    // Clear VGA screen
    {
        let mut vga = VGA_WRITER.lock();
        vga.clear_screen();
    }

    log::info!("Console initialized (VGA + Serial)");
}

/// Write a byte to both VGA and serial outputs
pub fn write_byte(byte: u8) {
    // Write to VGA
    {
        let mut vga = VGA_WRITER.lock();
        vga.write_byte(byte);
    }

    // Write to serial
    {
        let mut serial = SERIAL_WRITER.lock();
        if let Some(ref mut writer) = *serial {
            writer.write_byte(byte);
        }
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
    // Write to VGA
    VGA_WRITER.lock().write_fmt(args).unwrap();

    // Write to serial
    {
        let mut serial = SERIAL_WRITER.lock();
        if let Some(ref mut writer) = *serial {
            writer.write_fmt(args).unwrap();
        }
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
