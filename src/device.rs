use crate::instructions::instruction_id;
use crate::messaging::Messenger;
use crate::{ReadError, SerialPort, WriteError};
use core::time::Duration;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use crate::{Instruction, InstructionPacket};

/// Dynamixel [`Device`] for communicating with a [`Bus`].
pub struct Device<ReadBuffer, WriteBuffer, T: SerialPort> {
	messenger: Messenger<ReadBuffer, WriteBuffer, T>,
}

impl<ReadBuffer, WriteBuffer, T> core::fmt::Debug for Device<ReadBuffer, WriteBuffer, T>
where
	T: SerialPort + core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Device")
			.field("serial_port", &self.messenger.serial_port)
			.field("baud_rate", &self.messenger.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
impl Device<Vec<u8>, Vec<u8>, serial2::SerialPort> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let messenger = Messenger::with_buffers_and_baud_rate(port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { messenger })
	}

	/// Create a new device for an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(serial_port: serial2::SerialPort) -> std::io::Result<Self> {
		let messenger = Messenger::with_buffers(serial_port, vec![0; 128], vec![0; 128])?;
		Ok(Self { messenger })
	}
}

#[cfg(feature = "serial2")]
impl<ReadBuffer, WriteBuffer> Device<ReadBuffer, WriteBuffer, serial2::SerialPort>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<std::path::Path>,
		baud_rate: u32,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let messenger = Messenger::with_buffers_and_baud_rate(port, read_buffer, write_buffer, baud_rate);
		Ok(Self { messenger })
	}
}
impl<ReadBuffer, WriteBuffer, T> Device<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Create a new device using pre-allocated buffers.
	pub fn with_buffers(
		serial_port: T,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> Result<Self, T::Error> {
		let messenger = Messenger::with_buffers(serial_port, read_buffer, write_buffer)?;
		Ok(Device { messenger })
	}

	/// Get a reference to the underlying [`Transport`].
	///
	/// Note that performing any read or write with the [`Transport`] bypasses the read/write buffer of the device,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the device manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &T {
		&self.messenger.serial_port
	}

	/// Consume this device object to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the device object.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> T {
		self.messenger.serial_port
	}

	/// Get the baud rate of the device.
	pub fn baud_rate(&self) -> u32 {
		self.messenger.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), T::Error> {
		self.messenger.set_baud_rate(baud_rate)?;
		Ok(())
	}

	/// Read a single [`Instruction`] with borrowed data
	///
	/// Use [`Device::read_owned`] to received owned data
	pub fn read(&mut self, timeout: Duration) -> Result<Instruction<&[u8]>, ReadError<T::Error>> {
		let packet = self.read_instruction_packet_timeout(timeout)?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Read a single [`Instruction`] with borrowed data
	#[cfg(any(feature = "alloc", feature = "std"))]
	pub fn read_owned(&mut self, timeout: Duration) -> Result<Instruction<Vec<u8>>, ReadError<T::Error>> {
		let packet = self.read_instruction_packet_timeout(timeout)?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Write a status message to the device.
	pub fn write_status<F>(
		&mut self,
		packet_id: u8,
		error: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
	{
		self.messenger
			.write_status(packet_id, instruction_id::STATUS, error, parameter_count, encode_parameters)
	}

	/// Write an empty status message with an error code.
	pub fn write_status_error(&mut self, packet_id: u8, error: u8) -> Result<(), WriteError<T::Error>> {
		self.write_status(packet_id, error, 0, |_| {})
	}

	/// Write an empty status message.
	pub fn write_status_ok(&mut self, packet_id: u8) -> Result<(), WriteError<T::Error>> {
		self.write_status(packet_id, 0, 0, |_| {})
	}

	/// Read a single [`InstructionPacket`].
	pub fn read_instruction_packet_timeout(&mut self, timeout: Duration) -> Result<InstructionPacket, ReadError<T::Error>> {
		self.messenger.read_packet_response_timeout(timeout)
	}
}

