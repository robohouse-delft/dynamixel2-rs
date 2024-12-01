#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::time::Duration;
#[cfg(feature = "serial2")]
use std::path::Path;

use crate::bus::{Bus, StatusPacket};
use crate::instructions::instruction_id;
use crate::packet::STATUS_HEADER_SIZE;
use crate::serial_port::SerialPort;
use crate::{ReadError, TransferError, WriteError};


/// Client for the Dynamixel Protocol 2 communication.
///
/// Used to interact with devices on the bus.
pub struct Client<ReadBuffer, WriteBuffer, T: SerialPort> {
	bus: Bus<ReadBuffer, WriteBuffer, T>,
}
impl<ReadBuffer, WriteBuffer, T> core::fmt::Debug for Client<ReadBuffer, WriteBuffer, T>
where
	T: SerialPort + core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Client")
			.field("serial_port", &self.bus.serial_port)
			.field("baud_rate", &self.bus.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
impl Client<Vec<u8>, Vec<u8>, serial2::SerialPort> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<Path>, baud_rate: u32) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { bus })
	}

	/// Create a new client using an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(serial_port: serial2::SerialPort) -> std::io::Result<Self> {
		let bus = Bus::with_buffers(serial_port, vec![0; 128], vec![0; 128])?;
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2")]
impl<ReadBuffer, WriteBuffer> Client<ReadBuffer, WriteBuffer, serial2::SerialPort>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<Path>,
		baud_rate: u32,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(port, read_buffer, write_buffer, baud_rate);
		Ok(Self { bus })
	}
}

impl<ReadBuffer, WriteBuffer, T> Client<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Create a new client using pre-allocated buffers.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	pub fn with_buffers(
		serial_port: T,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> Result<Self, T::Error> {
		let bus = Bus::with_buffers(serial_port, read_buffer, write_buffer)?;
		Ok(Self { bus })
	}

	/// Get a reference to the underlying [`SerialPort`].
	///
	/// Note that performing any read or write with the [`SerialPort`] bypasses the read/write buffer of the bus,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the bus manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &T {
		&self.bus.serial_port
	}

	/// Consume the client to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the client.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> T {
		self.bus.serial_port
	}

	/// Get the baud rate of the bus.
	pub fn baud_rate(&self) -> u32 {
		self.bus.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), T::Error> {
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
	) -> Result<StatusPacket<'_>, TransferError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
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
	) -> Result<(), WriteError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
	{
		self.bus
			.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)
	}

	/// Read a raw status response from the bus with the given deadline.
	pub fn read_status_response_timeout(&mut self, timeout: Duration) -> Result<StatusPacket, ReadError<T::Error>> {
		let response: StatusPacket = self.bus.read_packet_response_timeout(timeout)?;

		crate::InvalidInstruction::check(response.instruction_id(), instruction_id::STATUS)?;
		crate::MotorError::check(response.error())?;
		Ok(response)
	}

	/// Read a raw status response with an automatically calculated timeout.
	///
	/// The read timeout is determined by the expected number of response parameters and the baud rate of the bus.
	pub fn read_status_response(&mut self, expected_parameters: u16) -> Result<StatusPacket, ReadError<T::Error>> {
		// Official SDK adds a flat 34 milliseconds, so lets just mimick that.
		let message_size = STATUS_HEADER_SIZE as u32 + u32::from(expected_parameters) + 2;
		let timeout = crate::bus::message_transfer_time(message_size, self.bus.baud_rate) + Duration::from_millis(34);
		self.read_status_response_timeout(timeout)
	}
}
