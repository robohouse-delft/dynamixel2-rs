//! The [`SerialPort`] trait is used to implementing the Dynamixel Protocol 2.0 communication interface.

#[cfg(feature = "serial2")]
pub mod serial2;

use crate::ReadError;
use ::std::time::Duration;

/// [`SerialPort`]s are used to communicate with the hardware via reading and writing data.
/// The Dynamixel Protocol 2.0 uses 8 bits char size, 1 stop bit, no parity.
pub trait SerialPort {
	/// The error type returned by the transport when reading, writing or setting the baud rate.
	type Error: core::fmt::Debug + core::fmt::Display;
	/// Get the current baud rate of the transport.
	fn baud_rate(&self) -> Result<u32, crate::InitializeError<Self::Error>>;
	/// Set the baud rate of the transport.
	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error>;
	/// Discard the input buffer of the transport. Maybe a no-op on some platforms.
	fn discard_input_buffer(&mut self) -> Result<(), Self::Error>;
	/// Returns available bytes to read, blocking until at least one byte is available or the timeout duration elapses.
	fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> Result<usize, ReadError<Self::Error>>;
	/// Write all bytes in the buffer to the transport.
	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;
}
