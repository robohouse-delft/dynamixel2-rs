//! Trait implementation using the `serial2` crate.

use std::time::{Duration, Instant};

impl crate::SerialPort for serial2::SerialPort {
	type Error = std::io::Error;

	type Instant = std::time::Instant;

	fn baud_rate(&self) -> Result<u32, Self::Error> {
		self.get_configuration()?
			.get_baud_rate()
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

	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		let timeout = deadline.checked_duration_since(Instant::now())
			.ok_or(std::io::ErrorKind::TimedOut)?;
		self.set_read_timeout(timeout)?;
		serial2::SerialPort::read(self, buffer)
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		serial2::SerialPort::write_all(self, buffer)
	}

	fn make_deadline(&self, timeout: Duration) -> Self::Instant {
		Instant::now() + timeout
	}

	fn is_timeout_error(error: &Self::Error) -> bool {
		error.kind() == std::io::ErrorKind::TimedOut
	}
}
