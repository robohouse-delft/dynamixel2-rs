use super::Client;
use crate::{instruction_id, Ping};
use crate::{Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Ping a specific motor by ID.
	///
	/// This will not work correctly if the motor ID is [`packet_id::BROADCAST`].
	/// Use [`Self::scan`] instead.
	pub async fn ping(&mut self, motor_id: u8) -> Result<Response<Ping>, TransferError<SerialPort::Error>> {
		let response = self.transfer_single(motor_id, instruction_id::PING, 0, 3, |_| Ok(())).await?;
		Ok(response.try_into()?)
	}

	/// Scan the bus for motors with a broadcast ping
	#[super::only_sync]
	pub fn scan<'a>(&'a mut self) -> Result<Scan<'a, SerialPort, Buffer>, crate::WriteError<SerialPort::Error>> {
		self.write_instruction(crate::packet_id::BROADCAST, instruction_id::PING, 0, |_| Ok(()))?;
		Ok(Scan { client: self })
	}
}

#[super::only_sync]
macro_rules! make_scan_struct {
	($($DefaultSerialPort:ty)?) => {
		/// A scan operation that returns [`Response<Ping>`] when iterated
		pub struct Scan<'a,SerialPort $(= $DefaultSerialPort)?, Buffer = crate::bus::DefaultBuffer>
		where
			SerialPort: super::SerialPort,
			Buffer: AsRef<[u8]> + AsMut<[u8]>,
		{
			client: &'a mut Client<SerialPort, Buffer>,
		}
	}
}

#[cfg(feature = "serial2")]
#[super::only_sync]
make_scan_struct!(serial2::SerialPort);

#[cfg(not(feature = "serial2"))]
#[super::only_sync]
make_scan_struct!();

#[super::only_sync]
impl<SerialPort, Buffer> core::fmt::Debug for Scan<'_, SerialPort, Buffer>
where
	SerialPort: super::SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Scan").field("serial_port", self.client.serial_port()).finish()
	}
}

#[super::only_sync]
impl<SerialPort, Buffer> Scan<'_, SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Scan for the next motor reply
	pub fn scan_next(&mut self) -> Option<Result<Response<Ping>, crate::ReadError<SerialPort::Error>>> {
		let response = self.next_response();
		match response {
			Ok(response) => Some(Ok(response)),
			Err(crate::ReadError::Io(e)) if SerialPort::is_timeout_error(&e) => {
				trace!("Ping response timed out.");
				None
			},
			Err(e) => Some(Err(e)),
		}
	}

	fn next_response(&mut self) -> Result<Response<Ping>, crate::ReadError<SerialPort::Error>> {
		let response_time = crate::bus::message_transfer_time(14, self.client.baud_rate());
		let timeout = response_time * 253 + core::time::Duration::from_millis(34);

		let response = self.client.read_status_response_timeout(timeout)?;
		Ok(response.try_into()?)
	}
}

#[super::only_sync]
impl<SerialPort, Buffer> Drop for Scan<'_, SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next().is_some() {}
	}
}

#[super::only_sync]
impl<SerialPort, Buffer> Iterator for Scan<'_, SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<Ping>, crate::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.scan_next()
	}
}
