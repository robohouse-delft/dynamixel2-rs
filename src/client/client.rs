use core::time::Duration;

use crate::ReadError;
use crate::TransferError;
use crate::WriteError;
use crate::bus::StatusPacket;
use crate::bus::instruction_id;
use super::Bus;
use super::SerialPort;

/// The default additional time added to the automatically calculated read timeout of a status response.
///
/// The official SDK adds a flat 34 milliseconds, so we mimic that.
const DEFAULT_RESPONSE_TIMEOUT_PADDING: Duration = Duration::from_millis(34);

/// Client for the Dynamixel Protocol 2 communication.
///
/// Used to interact with devices on the bus.
///
/// If a serial port backend is enabled, the `Port` generic type argument defaults to that backend's
/// serial port type: `serial2::SerialPort` with the `"serial2"` feature, or `serial2_tokio::SerialPort`
/// (for the asynchronous client) with the `"serial2-tokio"` feature.
/// If neither is enabled, the `Port` argument must always be specified.
///
/// The `Buffer` generic type argument defaults to `Vec<u8>` if the `"alloc"` feature is enabled,
/// and to `&'static mut [u8]` otherwise.
/// See the [`crate::static_buffer!()`] macro for a way to safely create a mutable static buffer.
pub struct Client<Port, Buffer = crate::bus::DefaultBuffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	bus: Bus<Port, Buffer>,

	/// Additional time added to the automatically calculated read timeout of a status response.
	response_timeout_padding: Duration,
}

impl<Port, Buffer> core::fmt::Debug for Client<Port, Buffer>
where
	Port: SerialPort + core::fmt::Debug,
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
#[super::only_sync]
impl Client<serial2::SerialPort, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let serial_port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}
}

#[cfg(feature = "serial2")]
#[super::only_sync]
impl<Buffer> Client<serial2::SerialPort, Buffer>
where
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<std::path::Path>,
		baud_rate: u32,
		read_buffer: Buffer,
		write_buffer: Buffer,
	) -> std::io::Result<Self> {
		let serial_port = serial2::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate);
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}
}

#[cfg(feature = "serial2-tokio")]
#[super::only_async]
impl Client<serial2_tokio::SerialPort, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let serial_port = serial2_tokio::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}
}

#[cfg(feature = "serial2-tokio")]
#[super::only_async]
impl<Buffer> Client<serial2_tokio::SerialPort, Buffer>
where
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<std::path::Path>,
		baud_rate: u32,
		read_buffer: Buffer,
		write_buffer: Buffer,
	) -> std::io::Result<Self> {
		let serial_port = serial2_tokio::SerialPort::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate);
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}
}

#[cfg(feature = "alloc")]
impl<Port> Client<Port, alloc::vec::Vec<u8>>
where
	Port: SerialPort,
{
	/// Create a new client using an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	#[cfg(feature = "alloc")]
	pub fn new(serial_port: Port) -> Result<Self, Port::Error> {
		let bus = Bus::with_buffers(serial_port, alloc::vec![0; 128], alloc::vec![0; 128])?;
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}
}

#[super::bisync]
impl<Port, Buffer> Client<Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Create a new client using pre-allocated buffers.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	pub fn with_buffers(serial_port: Port, read_buffer: Buffer, write_buffer: Buffer) -> Result<Self, Port::Error> {
		let bus = Bus::with_buffers(serial_port, read_buffer, write_buffer)?;
		Ok(Self {
			bus,
			response_timeout_padding: DEFAULT_RESPONSE_TIMEOUT_PADDING,
		})
	}

	/// Get a reference to the underlying serial port.
	///
	/// Note that performing any read or write to the serial port bypasses the read/write buffer of the bus,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the bus manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &Port {
		&self.bus.serial_port
	}

	/// Consume the client to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the client.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> Port {
		self.bus.serial_port
	}

	/// Get the baud rate of the bus.
	pub fn baud_rate(&self) -> u32 {
		self.bus.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Port::Error> {
		self.bus.set_baud_rate(baud_rate)
	}

	/// Get the additional time added to the automatically calculated read timeout of a status response.
	///
	/// This padding is added on top of the transfer time computed from the expected response size and baud rate by [`Self::read_status_response`].
	/// It defaults to 34 milliseconds, mimicking the official SDK.
	///
	/// Be aware that reducing this too much could cause unnecessary timeout errors.
	pub fn response_timeout_padding(&self) -> Duration {
		self.response_timeout_padding
	}

	/// Set the additional time added to the automatically calculated read timeout of a status response.
	///
	/// See [`Self::response_timeout_padding`] for more details.
	pub fn set_response_timeout_padding(&mut self, padding: Duration) {
		self.response_timeout_padding = padding;
	}

	/// Write a raw instruction to a stream, and read a single raw response.
	///
	/// This function also checks that the packet ID of the status response matches the one from the instruction.
	///
	/// This is not suitable for broadcast instructions.
	/// For broadcast instructions, each motor sends an individual response or no response is send at all.
	/// Instead, use [`Self::write_instruction`] and [`Self::read_status_response`].
	pub async fn transfer_single<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		expected_response_parameters: u16,
		encode_parameters: F,
	) -> Result<StatusPacket<'_>, TransferError<Port::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)
			.await?;
		let response = self.read_status_response(expected_response_parameters, true).await?;
		crate::error::InvalidPacketId::check(response.packet_id(), packet_id).map_err(crate::ReadError::from)?;
		Ok(response)
	}

	/// Write an instruction message to the bus.
	pub async fn write_instruction<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<Port::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.bus
			.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)
			.await
	}

	/// Read a raw status response from the bus with the given deadline.
	///
	/// If `check_error` is `true`, the packet error field is checked and a [`crate::MotorError`] is returned if it is set.
	/// Fast sync/bulk read responses pack a per-motor error byte for every motor into a single packet,
	/// and the first motor's error byte coincides with the packet error field, so those reads pass `false`
	/// and check the error of each motor individually while iterating the response instead.
	pub async fn read_status_response_timeout<'a>(
		&'a mut self,
		timeout: Duration,
		check_error: bool,
	) -> Result<StatusPacket<'a>, ReadError<Port::Error>> {
		let deadline = self.serial_port().make_deadline(timeout);
		let packet = self.bus.read_packet_deadline(deadline).await?;
		let status = match packet.as_status() {
			Some(status) => status,
			None => {
				return Err(crate::InvalidInstruction {
					actual: packet.instruction_id(),
					expected: instruction_id::STATUS,
				}
				.into())
			},
		};

		if check_error {
			crate::MotorError::check(status.error())?;
		}
		Ok(status)
	}

	/// Read a raw status response with an automatically calculated timeout.
	///
	/// The read timeout is determined by the expected number of response parameters and the baud rate of the bus.
	/// See [`Self::read_status_response_timeout`] for the meaning of `check_error`.
	pub async fn read_status_response<'a>(
		&'a mut self,
		expected_parameters: u16,
		check_error: bool,
	) -> Result<StatusPacket<'a>, ReadError<Port::Error>> {
		let message_size = crate::bus::StatusPacket::message_len(expected_parameters as usize) as u32;
		let timeout = crate::bus::message_transfer_time(message_size, self.bus.baud_rate) + self.response_timeout_padding;
		self.read_status_response_timeout(timeout, check_error).await
	}

	/// Read an empty response from the bus if the motor ID is not the broadcast ID.
	///
	/// If the motor ID is the broadcast ID, return a fake response from the broadcast ID.
	pub(crate) async fn read_response_if_not_broadcast(&mut self, motor_id: u8) -> Result<crate::Response<()>, ReadError<Port::Error>> {
		if motor_id == crate::bus::packet_id::BROADCAST {
			Ok(crate::Response {
				motor_id: crate::bus::packet_id::BROADCAST,
				alert: false,
				data: (),
			})
		} else {
			Ok(self.read_status_response(0, true).await?.try_into()?)
		}
	}
}
