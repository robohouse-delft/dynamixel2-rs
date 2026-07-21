use super::Client;
use super::SerialPort;
use crate::bus::instruction_id;
use crate::client::Ping;
use crate::{Response, TransferError};

#[super::bisync]
impl<Port, Buffer> Client<Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Ping a specific motor by ID.
	///
	/// This will not work correctly if the motor ID is [`crate::bus::packet_id::BROADCAST`].
	/// Use [`Self::scan`] instead.
	pub async fn ping(&mut self, motor_id: u8) -> Result<Response<Ping>, TransferError<Port::Error>> {
		let response = self.transfer_single(motor_id, instruction_id::PING, 0, 3, |_| Ok(())).await?;
		Ok(response.try_into()?)
	}

	/// Scan the bus for motors with a broadcast ping.
	///
	/// See [`Scan`] for how to consume the per-motor replies.
	pub async fn scan<'a>(&'a mut self) -> Result<Scan<'a, Port, Buffer>, crate::WriteError<Port::Error>> {
		self.write_instruction(crate::bus::packet_id::BROADCAST, instruction_id::PING, 0, |_| Ok(()))
			.await?;
		Ok(Scan { client: self })
	}
}

/// A scan operation that yields a [`Response`] for each motor that replies to the broadcast ping.
///
/// Scanning ends when no further reply arrives within the timeout. The replies must be fully consumed
/// before the client is used again. The synchronous client is an [`Iterator`] and drains any unread
/// replies on drop; the asynchronous client cannot (a [`Drop`] can't `.await`), so call
/// [`scan_next`](Self::scan_next) until it returns [`None`] — dropping it early corrupts the next
/// transaction.
#[super::bisync]
pub struct Scan<'a, Port, Buffer = crate::bus::DefaultBuffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<Port, Buffer>,
}

impl<Port, Buffer> core::fmt::Debug for Scan<'_, Port, Buffer>
where
	Port: SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Scan").field("serial_port", self.client.serial_port()).finish()
	}
}

#[super::bisync]
impl<Port, Buffer> Scan<'_, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Scan for the next motor reply, or [`None`] once no further reply arrives within the timeout.
	pub async fn scan_next(&mut self) -> Option<Result<Response<Ping>, crate::ReadError<Port::Error>>> {
		let response = self.next_response().await;
		match response {
			Ok(response) => Some(Ok(response)),
			Err(crate::ReadError::Io(e)) if Port::is_timeout_error(&e) => {
				trace!("Ping response timed out.");
				None
			},
			Err(e) => Some(Err(e)),
		}
	}

	async fn next_response(&mut self) -> Result<Response<Ping>, crate::ReadError<Port::Error>> {
		let response_time = crate::bus::message_transfer_time(14, self.client.baud_rate());
		let timeout = response_time * 253 + core::time::Duration::from_millis(34);
		let response = self.client.read_status_response_timeout(timeout, true).await?;
		Ok(response.try_into()?)
	}
}

// `Iterator` and `Drop` are synchronous-only: `Iterator::next` cannot `.await`, and `Drop` cannot
// drain the bus asynchronously. The async client uses `scan_next().await` instead.
#[super::only_sync]
impl<Port, Buffer> Drop for Scan<'_, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next().is_some() {}
	}
}

#[super::only_sync]
impl<Port, Buffer> Iterator for Scan<'_, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<Ping>, crate::ReadError<Port::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.scan_next()
	}
}
