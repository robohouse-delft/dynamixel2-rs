//! The [`Transport`] trait is used to implementing the Dynamixel Protocol 2.0 communication interface.

#[cfg(feature = "serial2")]
pub mod serial2;

use crate::ReadError;
use core::time::Duration;

/// [`Transport`]s are used to communicate with the hardware via reading and writing data.
///
/// The implementor of the trait must also configure the serial line to use 8 bits characters, 1 stop bit, no parity and no flow control.
pub trait Transport {
	/// The error type returned by the transport when reading, writing or setting the baud rate.
	type Error;

	/// A point in time that can be used as a deadline for a I/O operations.
	type Instant;

	/// Get the current baud rate of the transport.
	fn baud_rate(&self) -> Result<u32, crate::InitializeError<Self::Error>>;

	/// Set the baud rate of the transport.
	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error>;

	/// Discard the input buffer of the transport. Maybe a no-op on some platforms.
	fn discard_input_buffer(&mut self) -> Result<(), Self::Error>;

	/// Returns available bytes to read, blocking until at least one byte is available or the deadline expires.
	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, ReadError<Self::Error>>;

	/// Write all bytes in the buffer to the transport.
	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;

	/// Make a deadline to expire after the given timeout.
	fn make_deadline(&self, timeout: Duration) -> Self::Instant;
}
