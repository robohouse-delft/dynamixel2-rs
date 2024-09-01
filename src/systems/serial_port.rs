use crate::ReadError;
use std::fmt::Formatter;
use std::time::Duration;

pub struct SerialPort {
	port: serial2::SerialPort,
}

impl SerialPort {
	pub fn new(port: serial2::SerialPort) -> Self {
		Self { port }
	}
}

impl core::fmt::Debug for SerialPort {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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

impl crate::Transport for SerialPort {
	type Error = std::io::Error;

	fn baud_rate(&self) -> Result<u32, crate::InitializeError> {
		self.port
			.get_configuration()
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

	fn read(&mut self, buffer: &mut [u8], timeout: Duration) -> Result<usize, ReadError<Self::Error>> {
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
