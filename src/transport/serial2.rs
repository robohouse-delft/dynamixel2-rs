//! Serial port transport implementation using the `serial2` crate.

use crate::ReadError;
use std::time::Duration;

/// Re-exported `serial2` crate in case you need to modify serial port settings.
pub use serial2;

impl crate::Transport for serial2::SerialPort {
	type Error = std::io::Error;

	fn baud_rate(&self) -> Result<u32, crate::InitializeError<Self::Error>> {
		self.get_configuration()
			.map_err(crate::InitializeError::GetConfiguration)?
			.get_baud_rate()
			.map_err(crate::InitializeError::GetBaudRate)
	}

	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error> {
		let mut settings = self.get_configuration()?;
		settings.set_baud_rate(baud_rate)?;
		self.set_configuration(&settings)?;
		Ok(())
	}

	fn discard_input_buffer(&mut self) -> Result<(), Self::Error> {
		serial2::SerialPort::discard_input_buffer(self)
	}

	fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> Result<usize, ReadError<Self::Error>> {
		self.set_read_timeout(timeout).map_err(ReadError::Io)?;
		match serial2::SerialPort::read(self, buffer) {
			Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Err(ReadError::Timeout),
			Err(e) => Err(ReadError::Io(e)),
			Ok(count) => Ok(count),
		}
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		serial2::SerialPort::write_all(self, buffer)
	}
}
