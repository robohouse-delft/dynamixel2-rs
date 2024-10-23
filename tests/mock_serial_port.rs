use dynamixel2::SerialPort;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default, Clone)]
pub struct MockSerialPort {
	pub read_buffer: Arc<Mutex<VecDeque<u8>>>,
	pub write_buffer: Arc<Mutex<VecDeque<u8>>>,
	pub baud_rate: u32,
}

impl MockSerialPort {
	pub fn new(baud_rate: u32) -> Self {
		Self {
			read_buffer: Arc::new(Mutex::new(VecDeque::new())),
			write_buffer: Arc::new(Mutex::new(VecDeque::new())),
			baud_rate,
		}
	}

	pub fn device_port(&self) -> Self {
		MockSerialPort {
			read_buffer: self.write_buffer.clone(),
			write_buffer: self.read_buffer.clone(),
			baud_rate: self.baud_rate,
		}
	}
}

impl SerialPort for MockSerialPort {
	type Error = std::io::Error;

	type Instant = std::time::Instant;

	fn baud_rate(&self) -> Result<u32, Self::Error> {
		Ok(self.baud_rate)
	}

	fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Self::Error> {
		self.baud_rate = baud_rate;
		Ok(())
	}

	fn discard_input_buffer(&mut self) -> Result<(), Self::Error> {
		self.read_buffer.lock().unwrap().clear();
		Ok(())
	}

	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		let mut data = loop {
			if Instant::now() > *deadline {
				return Err(std::io::ErrorKind::TimedOut.into());
			}
			if let Ok(data) = self.read_buffer.try_lock() {
				break data;
			}
		};
		let len = buffer.len().min(data.len());
		buffer[..len].copy_from_slice(&data.drain(..len).collect::<Vec<u8>>());
		Ok(len)
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		let mut data = self.write_buffer.lock().unwrap();
		for &byte in buffer {
			data.push_back(byte);
		}
		Ok(())
	}

	fn make_deadline(&self, timeout: Duration) -> Self::Instant {
		Instant::now() + timeout
	}

	fn is_timeout_error(error: &Self::Error) -> bool {
		error.kind() == std::io::ErrorKind::TimedOut
	}
}
