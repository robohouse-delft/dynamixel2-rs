//! Trait implementation using the `serial2` crate.

use super::Serial2Port;
use std::time::{Duration, Instant};

#[super::super::bisync]
impl super::SerialPort for Serial2Port {
	type Error = std::io::Error;
	type Instant = std::time::Instant;

	fn baud_rate(&self) -> Result<u32, Self::Error> {
		self.get_configuration()?.get_baud_rate()
	}

	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error> {
		let mut settings = self.get_configuration()?;
		settings.set_baud_rate(baud_rate)?;
		self.set_configuration(&settings)?;
		Ok(())
	}

	fn discard_input_buffer(&mut self) -> Result<(), Self::Error> {
		Serial2Port::discard_input_buffer(self)
	}

	#[super::super::only_sync]
	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		let timeout = deadline
			.checked_duration_since(Instant::now())
			.ok_or(std::io::ErrorKind::TimedOut)?;
		self.set_read_timeout(timeout)?;
		Serial2Port::read(self, buffer)
	}

	#[super::super::only_async]
	async fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		let timeout = deadline
			.checked_duration_since(Instant::now())
			.ok_or(std::io::ErrorKind::TimedOut)?;
		tokio::time::timeout(timeout, Serial2Port::read(self, buffer))
			.await
			.map_err(|_| std::io::ErrorKind::TimedOut)?
	}

	async fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		Serial2Port::write_all(self, buffer).await
	}

	fn make_deadline(&self, timeout: Duration) -> Self::Instant {
		Instant::now() + timeout
	}

	fn is_timeout_error(error: &Self::Error) -> bool {
		error.kind() == std::io::ErrorKind::TimedOut
	}
}
