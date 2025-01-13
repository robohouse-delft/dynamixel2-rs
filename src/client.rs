use core::time::Duration;
#[cfg(feature = "serial2")]
use std::path::Path;

use crate::bus::{Bus, StatusPacket};
use crate::instructions::instruction_id;
use crate::{ReadError, TransferError, WriteError};

macro_rules! make_client_struct {
	($($DefaultSerialPort:ty)?) => {
		/// Client for the Dynamixel Protocol 2 communication.
		///
		/// Used to interact with devices on the bus.
		///
		/// If the `"serial2"` feature is enabled, the `SerialPort` generic type argument defaults to [`serial2::SerialPort`].
		/// If it is not enabled, the `SerialPort` argument must always be specified.
		///
		/// The `Buffer` generic type argument defaults to `Vec<u8>` if the `"alloc"` feature is enabled,
		/// and to `&'static mut [u8]` otherwise.
		/// See the [`static_buffer!()`] macro for a way to safely create a mutable static buffer.
		pub struct Client<SerialPort $(= $DefaultSerialPort)?, Buffer = crate::bus::DefaultBuffer>
		where
			SerialPort: crate::SerialPort,
			Buffer: AsRef<[u8]> + AsMut<[u8]>,
		{
			bus: Bus<SerialPort, Buffer>,
		}
	};
}

#[cfg(feature = "serial2")]
make_client_struct!(serial2::SerialPort);

#[cfg(not(feature = "serial2"))]
make_client_struct!();

impl<SerialPort, Buffer> core::fmt::Debug for Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Client")
			.field("serial_port", &self.bus.serial_port)
			.field("baud_rate", &self.bus.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
impl Client<serial2::SerialPort, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<Path>, baud_rate: u32) -> std::io::Result<Self> {
		let serial_port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(
			serial_port,
			vec![0; 128],
			vec![0; 128],
			baud_rate
		);
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2")]
impl<Buffer> Client<serial2::SerialPort, Buffer>
where
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<Path>,
		baud_rate: u32,
		read_buffer: Buffer,
		write_buffer: Buffer,
	) -> std::io::Result<Self> {
		let serial_port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(
			serial_port,
			read_buffer,
			write_buffer,
			baud_rate,
		);
		Ok(Self { bus })
	}
}

#[cfg(feature = "alloc")]
impl<SerialPort> Client<SerialPort, Vec<u8>>
where
	SerialPort: crate::SerialPort,
{
	/// Create a new client using an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	#[cfg(feature = "alloc")]
	pub fn new(serial_port: SerialPort) -> Result<Self, SerialPort::Error> {
		let bus = Bus::with_buffers(
			serial_port,
			vec![0; 128],
			vec![0; 128],
		)?;
		Ok(Self { bus })
	}
}

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Create a new client using pre-allocated buffers.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	pub fn with_buffers(
		serial_port: SerialPort,
		read_buffer: Buffer,
		write_buffer: Buffer,
	) -> Result<Self, SerialPort::Error> {
		let bus = Bus::with_buffers(
			serial_port,
			read_buffer,
			write_buffer,
		)?;
		Ok(Self { bus })
	}

	/// Get a reference to the underlying serial port.
	///
	/// Note that performing any read or write to the serial port bypasses the read/write buffer of the bus,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the bus manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &SerialPort {
		&self.bus.serial_port
	}

	/// Consume the client to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the client.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> SerialPort {
		self.bus.serial_port
	}

	/// Get the baud rate of the bus.
	pub fn baud_rate(&self) -> u32 {
		self.bus.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), SerialPort::Error> {
		self.bus.set_baud_rate(baud_rate)
	}

	/// Write a raw instruction to a stream, and read a single raw response.
	///
	/// This function also checks that the packet ID of the status response matches the one from the instruction.
	///
	/// This is not suitable for broadcast instructions.
	/// For broadcast instructions, each motor sends an individual response or no response is send at all.
	/// Instead, use [`Self::write_instruction`] and [`Self::read_status_response`].
	pub fn transfer_single<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		expected_response_parameters: u16,
		encode_parameters: F,
	) -> Result<StatusPacket<'_>, TransferError<SerialPort::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)?;
		let response = self.read_status_response(expected_response_parameters)?;
		crate::error::InvalidPacketId::check(response.packet_id(), packet_id).map_err(crate::ReadError::from)?;
		Ok(response)
	}

	/// Write an instruction message to the bus.
	pub fn write_instruction<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<SerialPort::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.bus
			.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)
	}

	/// Read a raw status response from the bus with the given deadline.
	pub fn read_status_response_timeout(
		&mut self,
		timeout: Duration,
	) -> Result<StatusPacket, ReadError<SerialPort::Error>>
	{
		let deadline = self.serial_port().make_deadline(timeout);
		let packet = self.bus.read_packet_deadline(deadline)?;
		let status = match packet.as_status() {
			Some(status) => status,
			None => return Err(crate::InvalidInstruction {
				actual: packet.instruction_id(),
				expected: instruction_id::STATUS,
			}.into()),
		};

		crate::MotorError::check(status.error())?;
		Ok(status)
	}

	/// Read a raw status response with an automatically calculated timeout.
	///
	/// The read timeout is determined by the expected number of response parameters and the baud rate of the bus.
	pub fn read_status_response(
		&mut self,
		expected_parameters: u16,
	) -> Result<StatusPacket, ReadError<SerialPort::Error>> {
		// Official SDK adds a flat 34 milliseconds, so lets just mimick that.
		let message_size = crate::bus::StatusPacket::message_len(expected_parameters as usize) as u32;
		let timeout = crate::bus::message_transfer_time(message_size, self.bus.baud_rate) + Duration::from_millis(34);
		self.read_status_response_timeout(timeout)
	}
}
