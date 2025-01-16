use core::time::Duration;

use super::{instruction_id, packet_id};
use crate::bus::StatusPacket;
use crate::{Client, ReadError, Response, TransferError};

/// A response from a motor to a ping instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Ping {
	/// The model of the motor.
	///
	/// Refer to the online manual to find the codes for each model.
	pub model: u16,

	/// The firmware version of the motor.
	pub firmware: u8,
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<Ping> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		let parameters = status_packet.parameters();
		crate::InvalidParameterCount::check(parameters.len(), 3)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: Ping {
				model: crate::bus::endian::read_u16_le(&parameters[0..]),
				firmware: crate::bus::endian::read_u8_le(&parameters[2..]),
			},
		})
	}
}

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Ping a specific motor by ID.
	///
	/// This will not work correctly if the motor ID is [`packet_id::BROADCAST`].
	/// Use [`Self::scan`] instead.
	pub fn ping(&mut self, motor_id: u8) -> Result<Response<Ping>, TransferError<SerialPort::Error>> {
		let response = self.transfer_single(motor_id, instruction_id::PING, 0, 3, |_| Ok(()))?;
		Ok(response.try_into()?)
	}

	/// Scan the bus for motors with a broadcast ping
	pub fn scan(&mut self) -> Result<Scan<SerialPort, Buffer>, crate::WriteError<SerialPort::Error>> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::PING, 0, |_| Ok(()))?;
		Ok(Scan { client: self })
	}
}

macro_rules! make_scan_struct {
	($($DefaultSerialPort:ty)?) => {
		/// A scan operation that returns [`Response<Ping>`] when iterated
		pub struct Scan<'a,SerialPort $(= $DefaultSerialPort)?, Buffer = crate::bus::DefaultBuffer>
		where
			SerialPort: crate::SerialPort,
			Buffer: AsRef<[u8]> + AsMut<[u8]>,
		{
			client: &'a mut Client<SerialPort, Buffer>,
		}
	}
}

#[cfg(feature = "serial2")]
make_scan_struct!(serial2::SerialPort);

#[cfg(not(feature = "serial2"))]
make_scan_struct!();
impl<SerialPort, Buffer> core::fmt::Debug for Scan<'_, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort + std::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Scan").field("serial_port", self.client.serial_port()).finish()
	}
}

impl<SerialPort, Buffer> Scan<'_, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Scan for the next motor reply
	pub fn scan_next(&mut self) -> Option<Result<Response<Ping>, crate::ReadError<SerialPort::Error>>> {
		let response = self.next_response();
		match response {
			Ok(response) => Some(Ok(response)),
			Err(ReadError::Io(e)) if SerialPort::is_timeout_error(&e) => {
				trace!("Ping response timed out.");
				None
			},
			Err(e) => Some(Err(e)),
		}
	}

	fn next_response(&mut self) -> Result<Response<Ping>, crate::ReadError<SerialPort::Error>> {
		let response_time = crate::bus::message_transfer_time(14, self.client.baud_rate());
		let timeout = response_time * 253 + Duration::from_millis(34);

		let response = self.client.read_status_response_timeout(timeout)?;
		Ok(response.try_into()?)
	}
}

impl<SerialPort, Buffer> Drop for Scan<'_, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next().is_some() {}
	}
}

impl<SerialPort, Buffer> Iterator for Scan<'_, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<Ping>, crate::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.scan_next()
	}
}
