use std::sync::{Arc, Mutex, MutexGuard};
use dynamixel2::SerialPort;
use std::time::{Duration, Instant};
use log::trace;

#[derive(Clone)]
pub struct SharedBuffer {
	buffer: Arc<Mutex<Vec<u8>>>,
}

impl SharedBuffer {
	pub fn new() -> SharedBuffer {
		SharedBuffer {
			buffer: Arc::new(Mutex::new(Vec::new())),
		}
	}

	pub fn read(&self) -> Option<MutexGuard<Vec<u8>>> {
		self.buffer.try_lock().ok()
	}

	pub fn write(&self, data: &[u8]) {
		let mut buffer = self.buffer.lock().ok().unwrap();
		buffer.extend_from_slice(data);
	}
}

#[derive(Clone)]
pub struct MockSerial {
	pub name: String,
	other_device_buffers: Vec<SharedBuffer>,
	pub read_buffer: SharedBuffer,
	baud_rate: u32,
}

impl MockSerial {
	pub fn new(name: &str) -> MockSerial {
		Self {
			name: name.to_string(),
			other_device_buffers: Vec::new(),
			read_buffer: SharedBuffer::new(),
			baud_rate: 56700,
		}
	}

	pub fn add_device(&mut self, serial: SharedBuffer) {
		self.other_device_buffers.push(serial);
	}
}

impl SerialPort for MockSerial {
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
		Ok(())
	}

	fn read(&mut self, buffer: &mut [u8], deadline: &Self::Instant) -> Result<usize, Self::Error> {
		loop {
			if Instant::now() > *deadline {
				return Err(std::io::ErrorKind::TimedOut.into());
			}
			if let Some(mut data) = self.read_buffer.read() {
				if data.is_empty() {
					continue
				}
				let len = data.len();
				if len > buffer.len() {
					panic!("buffer is too small");
				}
				buffer[..len].copy_from_slice(&data);
				data.clear();
				if !buffer[..len].is_empty() {
					trace!("{} read: {:?}", self.name, &buffer[..len]);
				}
				return Ok(len)
			}
		};
	}

	fn write_all(&mut self, buffer: &[u8]) -> Result<(), Self::Error> {
		self.other_device_buffers.iter().for_each(|sender| {
			sender.write(buffer);
		});
		Ok(())
	}

	fn make_deadline(&self, timeout: Duration) -> Self::Instant {
		Instant::now() + timeout
	}

	fn is_timeout_error(error: &Self::Error) -> bool {
		error.kind() == std::io::ErrorKind::TimedOut
	}

}
