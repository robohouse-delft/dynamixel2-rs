use crate::instructions::InstructionId;
use crate::messaging::Messenger;
use crate::{ReadError, Transport, WriteError};
use core::time::Duration;
use std::path::Path;

/// Dynamixel [`Device`] for communicating with a [`Bus`].
pub struct Device<ReadBuffer, WriteBuffer, T: Transport> {
	messenger: Messenger<ReadBuffer, WriteBuffer, T>,
}

impl<ReadBuffer, WriteBuffer, T> core::fmt::Debug for Device<ReadBuffer, WriteBuffer, T>
where
	T: Transport + core::fmt::Debug,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Device")
			.field("transport", &self.messenger.transport)
			.field("baud_rate", &self.messenger.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
impl Device<Vec<u8>, Vec<u8>, crate::transport::serial2::Serial2Port> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<Path>, baud_rate: u32) -> std::io::Result<Self> {
		let port = serial2::SerialPort::open(path, baud_rate)?;
		let messenger = Messenger::with_buffers_and_baud_rate(port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { messenger })
	}

	/// Create a new bus for an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(serial_port: serial2::SerialPort) -> Result<Self, crate::InitializeError<std::io::Error>> {
		let messenger = Messenger::with_buffers(serial_port, vec![0; 128], vec![0; 128])?;
		Ok(Self { messenger })
	}
}

#[cfg(feature = "serial2")]
impl<ReadBuffer, WriteBuffer> Device<ReadBuffer, WriteBuffer, crate::transport::serial2::Serial2Port>
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
		let messenger = Messenger::with_buffers_and_baud_rate(port, read_buffer, write_buffer, baud_rate);
		Ok(Self { messenger })
	}
}
impl<ReadBuffer, WriteBuffer, T> Device<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: Transport,
{
	/// Create a new bus using pre-allocated buffers.
	pub fn with_buffers(
		transport: impl Into<T>,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> Result<Self, crate::InitializeError<T::Error>> {
		let messenger = Messenger::with_buffers(transport, read_buffer, write_buffer)?;
		Ok(Device { messenger })
	}

	/// Get a reference to the underlying [`Transport`].
	///
	/// Note that performing any read or write with the [`Transport`] bypasses the read/write buffer of the bus,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the bus manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn transport(&self) -> &T {
		&self.messenger.transport
	}

	/// Consume this bus object to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the bus object.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_transport(self) -> T {
		self.messenger.transport
	}

	/// Get the baud rate of the bus.
	pub fn baud_rate(&self) -> u32 {
		self.messenger.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), T::Error> {
		self.messenger.set_baud_rate(baud_rate)?;
		Ok(())
	}

	// TODO: not implemented as how do you device on_response what type of response to send
	// pub fn transfer_single<R, F>(
	//     &mut self,
	//     timeout: Duration,
	//     on_response: R
	// ) -> Result<(), TransferError<T::Error>>
	// where
	//     R: FnOnce(InstructionPacket) -> (u8, u8, usize, F),
	//     F: FnOnce(&mut [u8])
	// {
	//     let packet = self.read_instruction_packet_timeout(timeout)?;
	//     let (packet_id, error, parameter_count, encode_parameters) = on_response(packet);
	//     self.write_status(
	//         packet_id,
	//         error,
	//         parameter_count,
	//         encode_parameters,
	//     )?;
	//     Ok(())
	// }

	/// Write a status message to the bus.
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
			.write_status(packet_id, InstructionId::Status.into(), error, parameter_count, encode_parameters)
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

/// [`InstructionPacket`] is a packet that contains an instruction and its parameters. Sent from the [`Bus`] to [`Device`]s.
#[derive(Debug)]
pub struct InstructionPacket<'a> {
	pub(crate) data: &'a [u8],
}
