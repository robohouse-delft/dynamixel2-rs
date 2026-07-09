#![allow(unused_imports)]

use core::marker::PhantomData;

use super::Client;
use super::SerialPort;
use crate::bus::data::{decode_status_packet_bytes, decode_status_packet_bytes_borrow};
use crate::client::BulkReadData;
use crate::{ReadError, Response, WriteError};

#[super::bisync]
impl<Port, Buffer> Client<Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read arbitrary data ranges from multiple motors.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// See [`BulkReadBytes`] for how to consume the per-motor replies.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub async fn bulk_read_bytes<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<BulkReadBytes<'a, T, Port, Buffer>, WriteError<Port::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		write_bulk_read_instruction(self, reads).await?;

		Ok(BulkReadBytes {
			client: self,
			bulk_read_data: reads,
			index: 0,
			data: PhantomData,
		})
	}

	/// Read arbitrary data ranges from multiple motors, borrowing the response from the internal read buffer.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub async fn bulk_read_bytes_borrow<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<BulkReadBytes<'a, T, Port, Buffer>, WriteError<Port::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		write_bulk_read_instruction(self, reads).await?;

		Ok(BulkReadBytes {
			client: self,
			bulk_read_data: reads,
			index: 0,
			data: PhantomData,
		})
	}
}

/// Write a bulk read instruction to a bus.
///
/// # Panic
/// Panics if multiple read operation use the same motor ID.
#[super::bisync]
async fn write_bulk_read_instruction<Port, Buffer>(
	client: &mut Client<Port, Buffer>,
	reads: &[BulkReadData],
) -> Result<(), WriteError<Port::Error>>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	for i in 0..reads.len() {
		for j in i + 1..reads.len() {
			if reads[i].motor_id == reads[j].motor_id {
				panic!(
					"bulk_read_cb: motor ID {} used multiple at index {} and {}",
					reads[i].motor_id, i, j
				)
			}
		}
	}
	client
		.write_instruction(
			crate::bus::packet_id::BROADCAST,
			crate::bus::instruction_id::BULK_READ,
			5 * reads.len(),
			|buffer| {
				for (i, read) in reads.iter().enumerate() {
					let buffer = &mut buffer[i * 5..][..5];
					crate::bus::endian::write_u8_le(&mut buffer[0..], read.motor_id);
					crate::bus::endian::write_u16_le(&mut buffer[1..], read.address);
					crate::bus::endian::write_u16_le(&mut buffer[3..], read.count);
				}
				Ok(())
			},
		)
		.await
}

/// A bulk read operation that returns the unparsed bytes from each motor, one reply at a time.
///
/// The replies must be fully consumed before the client is used again. The synchronous client is an
/// [`Iterator`] and drains any unread replies on drop; the asynchronous client cannot (a [`Drop`] can't
/// `.await`), so call [`read_next`](Self::read_next) until it returns [`None`] — dropping it early
/// corrupts the next transaction.
pub struct BulkReadBytes<'a, T, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<Port, Buffer>,
	bulk_read_data: &'a [BulkReadData],
	index: usize,
	data: PhantomData<fn() -> T>,
}

impl<T, Port, Buffer> core::fmt::Debug for BulkReadBytes<'_, T, Port, Buffer>
where
	Port: SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BulkReadBytes")
			.field("serial_port", self.client.serial_port())
			.field("bulk_read_data", &self.bulk_read_data)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

// The read methods are bisync: on the async client they are `async fn`s; on the sync client the
// `Iterator` impl below drives them. `remaining`/`pop_bulk_read_data` carry no bus I/O, so they
// are emitted unchanged in both flavours.
#[super::bisync]
impl<T, Port, Buffer> BulkReadBytes<'_, T, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.bulk_read_data.len() - self.index
	}

	fn pop_bulk_read_data(&mut self) -> Option<BulkReadData> {
		let data = self.bulk_read_data.get(self.index)?;
		self.index += 1;
		Some(*data)
	}

	/// Read the next motor reply, or [`None`] once every motor has replied.
	pub async fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<Port::Error>>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		let BulkReadData { motor_id, count, .. } = self.pop_bulk_read_data()?;
		Some(self.next_response(motor_id, count).await)
	}

	/// Read the next motor reply borrowing the data from the internal read buffer, or [`None`] once every motor has replied.
	pub async fn read_next_borrow(&mut self) -> Option<Result<Response<&T>, ReadError<Port::Error>>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let BulkReadData { motor_id, count, .. } = self.pop_bulk_read_data()?;
		Some(self.next_response_borrow(motor_id, count).await)
	}

	async fn next_response(&mut self, motor_id: u8, count: u16) -> Result<Response<T>, ReadError<Port::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		let response = self.client.read_status_response(count).await?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), count.into())?;
		Ok(decode_status_packet_bytes(response)?)
	}

	async fn next_response_borrow(&mut self, motor_id: u8, count: u16) -> Result<Response<&T>, ReadError<Port::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let response = self.client.read_status_response(count).await?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), count.into())?;
		Ok(decode_status_packet_bytes_borrow(response)?)
	}
}

// `Iterator` and `Drop` are synchronous-only: `Iterator::next` cannot `.await`, and `Drop` cannot
// drain the bus asynchronously. The async client uses `read_next().await` and relies on the next
// `write_instruction` discarding any unread responses.
#[super::only_sync]
impl<T, Port, Buffer> Iterator for BulkReadBytes<'_, T, Port, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<T>, ReadError<Port::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

#[super::only_sync]
impl<T, Port, Buffer> Drop for BulkReadBytes<'_, T, Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		for data in &self.bulk_read_data[self.index..] {
			self.client.read_status_response(data.count).ok();
		}
	}
}
