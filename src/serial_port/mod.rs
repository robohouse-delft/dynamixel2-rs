//! [`SerialPort`] trait to support reading/writing from different serial port implementations.

use core::time::Duration;

#[cfg(feature = "serial2")]
pub mod serial2;

/// [`SerialPort`]s are used to communicate with the hardware by reading and writing data.
///
/// The implementor of the trait must also configure the serial line to use 8 bits characters, 1 stop bit, no parity and no flow control.
pub trait SerialPort {
	/// The error type returned by the serial port when reading, writing or setting the baud rate.
	type Error;

	/// A point in time that can be used as a deadline for a I/O operations.
	type Instant: Copy;

	/// Get the current baud rate of the serial port.
	fn baud_rate(&self) -> Result<u32, Self::Error>;

	/// Set the baud rate of the serial port.
	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error>;

	/// Discard the input buffer of the serial port. Maybe a no-op on some platforms.
	fn discard_input_buffer(&mut self) -> Result<(), Self::Error>;

	/// Returns available bytes to read, blocking until at least one byte is available or the deadline expires.
	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error>;

	/// Write all bytes in the buffer to the serial port.
	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;

	/// Make a deadline to expire after the given timeout.
	fn make_deadline(&self, timeout: Duration) -> Self::Instant;

	/// Check if an error indicates a timeout.
	fn is_timeout_error(error: &Self::Error) -> bool;
}
