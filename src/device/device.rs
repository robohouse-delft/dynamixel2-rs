use super::Bus;
use super::SerialPort;
use crate::device::Instruction;
use crate::{ReadError, WriteError};
use core::time::Duration;

/// Dynamixel [`Device`] for implementing the device side of the DYNAMIXEL Protocol 2.0.
///
/// If a serial port backend is enabled, the `Port` generic type argument defaults to that backend's
/// serial port type: `serial2::SerialPort` with the `"serial2"` feature, or `serial2_tokio::SerialPort`
/// (for the asynchronous device) with the `"serial2-tokio"` feature.
/// If neither is enabled, the `Port` argument must always be specified.
///
/// The `Buffer` generic type argument defaults to `Vec<u8>` if the `"alloc"` feature is enabled,
/// and to `&'static mut [u8]` otherwise.
/// See the [`crate::static_buffer!()`] macro for a way to safely create a mutable static buffer.
pub struct Device<Port, Buffer = crate::bus::DefaultBuffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	bus: Bus<Port, Buffer>,
}

impl<Port, Buffer> core::fmt::Debug for Device<Port, Buffer>
where
	Port: SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Device")
			.field("serial_port", &self.bus.serial_port)
			.field("baud_rate", &self.bus.baud_rate)
			.finish_non_exhaustive()
	}
}

#[cfg(feature = "serial2")]
#[super::only_sync]
impl Device<serial2::SerialPort, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let serial_port = <serial2::SerialPort>::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2")]
#[super::only_sync]
impl<Buffer> Device<serial2::SerialPort, Buffer>
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
		let serial_port = <serial2::SerialPort>::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate);
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2-tokio")]
#[super::only_async]
impl Device<serial2_tokio::SerialPort, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<std::path::Path>, baud_rate: u32) -> std::io::Result<Self> {
		let serial_port = <serial2_tokio::SerialPort>::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, vec![0; 128], vec![0; 128], baud_rate);
		Ok(Self { bus })
	}
}

#[cfg(feature = "serial2-tokio")]
#[super::only_async]
impl<Buffer> Device<serial2_tokio::SerialPort, Buffer>
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
		let serial_port = <serial2_tokio::SerialPort>::open(path, baud_rate)?;
		let bus = Bus::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate);
		Ok(Self { bus })
	}
}


#[cfg(feature = "alloc")]
impl<Port> Device<Port, alloc::vec::Vec<u8>>
where
	Port: SerialPort,
{
	/// Create a new device for an open serial port.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(serial_port: Port) -> Result<Self, Port::Error> {
		let bus = Bus::with_buffers(serial_port, alloc::vec![0; 128], alloc::vec![0; 128])?;
		Ok(Self { bus })
	}
}

#[super::bisync]
impl<Port, Buffer> Device<Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Create a new device using pre-allocated buffers.
	pub fn with_buffers(serial_port: Port, read_buffer: Buffer, write_buffer: Buffer) -> Result<Self, Port::Error> {
		let bus = Bus::with_buffers(serial_port, read_buffer, write_buffer)?;
		Ok(Device { bus })
	}

	/// Get a reference to the underlying serial port.
	///
	/// Note that performing any read or write to the serial port bypasses the read/write buffer of the device,
	/// and may disrupt the communication with the motors.
	/// In general, it should be safe to read and write to the device manually in between instructions,
	/// if the response from the motors has already been received.
	pub fn serial_port(&self) -> &Port {
		&self.bus.serial_port
	}

	/// Consume this device object to get ownership of the serial port.
	///
	/// This discards any data in internal the read buffer of the device object.
	/// This is normally not a problem, since all data in the read buffer is also discarded when transmitting a new command.
	pub fn into_serial_port(self) -> Port {
		self.bus.serial_port
	}

	/// Get the baud rate of the device.
	pub fn baud_rate(&self) -> u32 {
		self.bus.baud_rate
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), Port::Error> {
		self.bus.set_baud_rate(baud_rate)?;
		Ok(())
	}

	/// Read a single [`Instruction`] with borrowed data
	///
	/// Use [`Device::read_owned`] to received owned data
	pub async fn read(&mut self, timeout: Duration) -> Result<Instruction<&[u8]>, ReadError<Port::Error>> {
		let packet = self.read_raw_instruction_timeout(timeout).await?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Read a single [`Instruction`] with borrowed data
	#[cfg(any(feature = "alloc", feature = "std"))]
	pub async fn read_owned(&mut self, timeout: Duration) -> Result<Instruction<alloc::vec::Vec<u8>>, ReadError<Port::Error>> {
		let packet = self.read_raw_instruction_timeout(timeout).await?;
		let packet = packet.try_into()?;
		Ok(packet)
	}

	/// Write a status message to the device.
	pub async fn write_status<F>(
		&mut self,
		packet_id: u8,
		error: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<Port::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.bus.write_status(packet_id, error, parameter_count, encode_parameters).await
	}

	/// Write an empty status message with an error code.
	pub async fn write_status_error(&mut self, packet_id: u8, error: u8) -> Result<(), WriteError<Port::Error>> {
		self.write_status(packet_id, error, 0, |_| Ok(())).await
	}

	/// Write an empty status message.
	pub async fn write_status_ok(&mut self, packet_id: u8) -> Result<(), WriteError<Port::Error>> {
		self.write_status(packet_id, 0, 0, |_| Ok(())).await
	}

	/// Read a single [`crate::bus::InstructionPacket`].
	pub async fn read_raw_instruction_timeout(
		&mut self,
		timeout: Duration,
	) -> Result<crate::bus::InstructionPacket<'_>, ReadError<Port::Error>> {
		let deadline = Port::make_deadline(self.serial_port(), timeout);
		let packet = self.bus.read_packet_deadline(deadline).await?;
		Ok(packet.as_instruction())
	}
}
