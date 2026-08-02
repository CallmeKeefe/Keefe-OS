//! Hardware Timer Implementation (PIT - Programmable Interval Timer)
//!
//! This module configures the Intel 8253/8254 PIT to generate periodic interrupts
//! at a fixed frequency (default 100Hz = 10ms tick).
//!
//! # Why PIT?
//!
//! For initial development, we use the legacy PIT because:
//! 1. **Universal Availability**: Present on all x86_64 hardware
//! 2. **Simplicity**: Easy to program via I/O ports
//! 3. **Reliability**: Well-understood behavior, no complex APIC setup
//!
//! In production, you would migrate to:
//! - Local APIC Timer (per-core, more precise)
//! - HPET (High Precision Event Timer) for better accuracy
//!
//! # PIT Architecture
//!
//! The PIT has 3 independent channels:
//! - Channel 0: Connected to IRQ0 (system timer) - **We use this**
//! - Channel 1: Legacy DRAM refresh (not used in modern systems)
//! - Channel 2: Connected to PC speaker (legacy audio)
//!
//! Each channel can be programmed with a divisor that divides the 1.193182 MHz
//! base clock to produce the desired interrupt frequency.
//!
//! # Frequency Calculation
//!
//! ```text
//! Divisor = PIT_BASE_FREQUENCY / desired_frequency
//! Divisor = 1193182 Hz / 100 Hz = 11932 (for 100Hz)
//! Divisor = 1193182 Hz / 1000 Hz = 1193 (for 1kHz)
//! ```
//!
//! # Capability-Based Security Note
//!
//! Access to the PIT is restricted to kernel mode (Ring 0).
//! User-space processes cannot directly program the timer.
//! Instead, they will receive timer notifications via IPC from
//! a user-space timer service (future implementation).

#![no_std]

use x86_64::instructions::port::{Port, PortWriteOnly};
use crate::console::println;

/// PIT Channel 0 data port (read/write divisor)
const PIT_DATA_CHANNEL_0: u16 = 0x40;

/// PIT Command/Mode register port
const PIT_COMMAND_REGISTER: u16 = 0x43;

/// Base frequency of the PIT oscillator (1.193182 MHz)
pub const PIT_BASE_FREQUENCY: u32 = 1193182;

/// Default timer frequency in Hz (100Hz = 10ms tick rate)
///
/// Trade-offs:
/// - Higher frequency (e.g., 1000Hz): More responsive scheduling, higher overhead
/// - Lower frequency (e.g., 50Hz): Less overhead, coarser scheduling granularity
///
/// 100Hz is a good balance for general-purpose systems.
pub const DEFAULT_TIMER_FREQUENCY: u32 = 100;

/// Initialize the PIT to generate interrupts at the specified frequency
///
/// # Configuration
///
/// We configure Channel 0 with:
/// - **Access Mode**: Low byte then high byte (16-bit divisor)
/// - **Operating Mode**: Square wave generator (Mode 3)
///   - Produces a symmetric square wave
///   - Interrupts occur at regular intervals
/// - **BCD/Binary**: Binary mode (16-bit binary divisor)
///
/// # Arguments
///
/// * `frequency` - Desired interrupt frequency in Hz (typically 100-1000)
///
/// # Panics
///
/// Panics if the frequency is too high (divisor would overflow 16 bits)
/// or too low (divisor would be zero).
pub fn init_pit(frequency: u32) {
    // Validate frequency range
    if frequency == 0 || frequency > PIT_BASE_FREQUENCY {
        panic!(
            "Invalid PIT frequency: {} Hz (must be 1-{} Hz)",
            frequency, PIT_BASE_FREQUENCY
        );
    }
    
    // Calculate the divisor
    let divisor = PIT_BASE_FREQUENCY / frequency;
    
    // Ensure divisor fits in 16 bits (max 65535)
    if divisor > 0xFFFF {
        panic!(
            "PIT frequency {} Hz is too low (divisor {} exceeds 16 bits)",
            frequency, divisor
        );
    }
    
    println!("  [PIT] Configuring for {} Hz (divisor = {})", frequency, divisor);
    
    // ================================================================
    // Program the PIT Command Register
    // ================================================================
    //
    // Command byte format (8 bits):
    // Bits 7-6: Select channel (00 = Channel 0)
    // Bits 5-4: Access mode (11 = Low byte, then high byte)
    // Bits 3-1: Operating mode (110 = Square wave generator, Mode 3)
    // Bit 0: BCD/Binary (0 = 16-bit binary)
    //
    // Result: 0b00110110 = 0x36
    
    let mut command_port: PortWriteOnly<u8> = Port::new(PIT_COMMAND_REGISTER);
    
    unsafe {
        // Send command byte: Channel 0, LSB+MSB, Square Wave, Binary
        command_port.write(0x36);
        
        // ================================================================
        // Program the Divisor (Low Byte first, then High Byte)
        // ================================================================
        let mut data_port: PortWriteOnly<u8> = Port::new(PIT_DATA_CHANNEL_0);
        
        // Write low byte of divisor
        data_port.write((divisor & 0xFF) as u8);
        
        // Write high byte of divisor
        data_port.write(((divisor >> 8) & 0xFF) as u8);
    }
    
    println!(
        "  [PIT] Timer initialized: {} Hz (period = {} ms)",
        frequency,
        1000 / frequency
    );
}

/// Reconfigure the PIT frequency at runtime
///
/// # Warning
///
/// Changing the frequency during operation may cause timing issues
/// with the scheduler and other time-dependent subsystems.
/// Only call this before enabling timer interrupts.
pub fn set_frequency(frequency: u32) {
    init_pit(frequency);
}

/// Get the current configured frequency
///
/// Note: This returns the last configured frequency, not the actual
/// hardware state. For accurate timing, track elapsed ticks separately.
pub const fn get_frequency() -> u32 {
    DEFAULT_TIMER_FREQUENCY
}

/// Get the timer period in milliseconds
pub const fn get_period_ms() -> u32 {
    1000 / DEFAULT_TIMER_FREQUENCY
}

/// PIT Register access structures for type-safe I/O
///
/// These wrappers provide safe access to PIT registers
/// using Rust's type system to prevent accidental writes.

#[allow(dead_code)]
struct PitChannel0 {
    data: Port<u8>,
    command: PortWriteOnly<u8>,
}

impl PitChannel0 {
    /// Create a new PIT Channel 0 accessor
    const fn new() -> Self {
        Self {
            data: Port::new(PIT_DATA_CHANNEL_0),
            command: Port::new(PIT_COMMAND_REGISTER),
        }
    }
    
    /// Read the current counter value (latched)
    ///
    /// # Safety
    ///
    /// Reading the counter requires latching it first via the command port.
    /// This is rarely needed in normal operation.
    unsafe fn read_counter(&mut self) -> u16 {
        let low = self.data.read();
        let high = self.data.read();
        (high as u16) << 8 | (low as u16)
    }
}

/// Static instance for PIT access (if needed for debugging)
#[allow(dead_code)]
static mut PIT: Option<PitChannel0> = None;

/// Initialize PIT access structure (for debugging)
#[allow(dead_code)]
pub fn init_pit_debug() {
    unsafe {
        PIT = Some(PitChannel0::new());
    }
}
