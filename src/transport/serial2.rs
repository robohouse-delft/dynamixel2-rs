//! Serial port transport implementation using the `serial2` crate.

use crate::ReadError;
use std::time::{Duration, Instant};

/// Re-exported `serial2` crate in case you need to modify serial port settings.
pub use serial2;

/// A wrapper around a `serial2::SerialPort` that implements the `Transport` trait.
pub struct Serial2Port {
	/// The serial port.
	pub port: serial2::SerialPort,
	/// The deadline for the next read operation.
	pub deadline: Option<Instant>,
}

impl Serial2Port {
	/// Create a new `Serial2Port` from a `serial2::SerialPort`.
	pub fn new(port: serial2::SerialPort) -> Self {
		Self {
			port,
			deadline: None,
		}
	}
}

impl From<serial2::SerialPort> for Serial2Port {
	fn from(port: serial2::SerialPort) -> Self {
		Self::new(port)
	}
}

impl core::fmt::Debug for Serial2Port {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> std::fmt::Result {
		#[derive(Debug)]
		#[allow(dead_code)] // Dead code analysis ignores derive debug impls, but that is the whole point of this struct.
		enum Raw {
			#[cfg(unix)]
			Fd(std::os::unix::io::RawFd),
			#[cfg(windows)]
			Handle(std::os::windows::io::RawHandle),
		}

		#[cfg(unix)]
		let raw = {
			use std::os::unix::io::AsRawFd;
			Raw::Fd(self.port.as_raw_fd())
		};
		#[cfg(windows)]
		let raw = {
			use std::os::windows::io::AsRawHandle;
			Raw::Handle(self.port.as_raw_handle())
		};
		write!(f, "SerialPort({:?})", raw)
	}
}

impl crate::Transport for Serial2Port {
	type Error = std::io::Error;

	fn baud_rate(&self) -> Result<u32, crate::InitializeError<Self::Error>> {
		self.port.get_configuration()
			.map_err(crate::InitializeError::GetConfiguration)?
			.get_baud_rate()
			.map_err(crate::InitializeError::GetBaudRate)
	}

	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error> {
		let mut settings = self.port.get_configuration()?;
		settings.set_baud_rate(baud_rate)?;
		self.port.set_configuration(&settings)?;
		Ok(())
	}

	fn discard_input_buffer(&mut self) -> Result<(), Self::Error> {
		self.port.discard_input_buffer()
	}

	fn set_timeout(&mut self, timeout: Duration) -> Result<(), Self::Error> {
		self.deadline = Some(Instant::now() + timeout);
		Ok(())
	}

	fn read(&mut self, buffer: &mut [u8]) -> Result<usize, ReadError<Self::Error>> {
		let timeout = self.deadline.ok_or(ReadError::Timeout)?.checked_duration_since(Instant::now()).ok_or(ReadError::Timeout)?;
		self.port.set_read_timeout(timeout).map_err(ReadError::Io)?;
		match self.port.read(buffer) {
			Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(ReadError::Timeout),
			Err(e) => Err(ReadError::Io(e)),
			Ok(count) => Ok(count),
		}
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		self.port.write_all(buffer)
	}
}
