//! Serial port transport implementation using the `serial2` crate.

use std::time::{Duration, Instant};

/// Re-exported `serial2` crate in case you need to modify serial port settings.
pub use serial2;

/// A wrapper around a `serial2::SerialPort` that implements the `Transport` trait.
pub struct Serial2Port {
	/// The serial port.
	pub port: serial2::SerialPort,
}

impl Serial2Port {
	/// Create a new `Serial2Port` from a `serial2::SerialPort`.
	pub fn new(port: serial2::SerialPort) -> Self {
		Self {
			port,
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

	type Instant = std::time::Instant;

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

	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		let timeout = deadline.checked_duration_since(Instant::now())
			.ok_or(std::io::ErrorKind::TimedOut)?;
		self.port.set_read_timeout(timeout)?;
		self.port.read(buffer)
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		self.port.write_all(buffer)
	}

	fn make_deadline(&self, timeout: Duration) -> Self::Instant {
		Instant::now() + timeout
	}

	fn is_timeout_error(error: &Self::Error) -> bool {
		error.kind() == std::io::ErrorKind::TimedOut
	}
}
