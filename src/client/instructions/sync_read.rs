#![allow(unused_imports)]

use core::marker::PhantomData;

use super::Client;
use super::SerialPort;
use crate::bus::data::Data;
use crate::bus::data::{decode_status_packet, decode_status_packet_bytes, decode_status_packet_bytes_borrow};
use crate::bus::endian::write_u16_le;
use crate::bus::{instruction_id, packet_id};
use crate::{ReadError, Response, WriteError};

#[super::bisync]
impl<Port, Buffer> Client<Port, Buffer>
where
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read a number of bytes from multiple motors in one command.
	///
	/// See [`SyncReadBytes`] for how to consume the per-motor replies.
	pub async fn sync_read_bytes<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<SyncReadBytes<'a, T, Port, Buffer>, WriteError<Port::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})
		.await?;

		Ok(SyncReadBytes {
			client: self,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}

	/// Read a number of bytes from multiple motors in one command, borrowing each reply from the read buffer.
	pub async fn sync_read_bytes_borrow<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<SyncReadBytes<'a, T, Port, Buffer>, WriteError<Port::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})
		.await?;

		Ok(SyncReadBytes {
			client: self,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}

	/// Read values from multiple motors in one command.
	///
	/// See [`SyncRead`] for how to consume the per-motor replies.
	pub async fn sync_read<'a, T: Data>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<SyncRead<'a, T, Port, Buffer>, WriteError<Port::Error>> {
		let count = T::ENCODED_SIZE;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})
		.await?;

		Ok(SyncRead {
			client: self,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}
}

/// A sync read operation that returns the unparsed bytes from each motor, one reply at a time.
///
/// The replies must be fully consumed before the client is used again. The synchronous client is an
/// [`Iterator`] and drains any unread replies on drop; the asynchronous client cannot (a [`Drop`] can't
/// `.await`), so call [`read_next`](Self::read_next) until it returns [`None`] — dropping it early
/// corrupts the next transaction.
#[super::bisync]
pub struct SyncReadBytes<'a, T, Port, Buffer = crate::bus::DefaultBuffer>
where
	T: ?Sized,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<Port, Buffer>,
	count: u16,
	motor_ids: &'a [u8],
	index: usize,
	data: PhantomData<fn() -> T>,
}

/// A sync read operation that returns the parsed value from each motor, one reply at a time.
///
/// The replies must be fully consumed before the client is used again. The synchronous client is an
/// [`Iterator`] and drains any unread replies on drop; the asynchronous client cannot (a [`Drop`] can't
/// `.await`), so call [`read_next`](Self::read_next) until it returns [`None`] — dropping it early
/// corrupts the next transaction.
#[super::bisync]
pub struct SyncRead<'a, T, Port, Buffer = crate::bus::DefaultBuffer>
where
	T: Data,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<Port, Buffer>,
	motor_ids: &'a [u8],
	index: usize,
	data: PhantomData<fn() -> T>,
}

impl<T, Port, Buffer> core::fmt::Debug for SyncReadBytes<'_, T, Port, Buffer>
where
	Port: SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SyncReadBytes")
			.field("serial_port", self.client.serial_port())
			.field("motor_ids", &self.motor_ids)
			.field("count", &self.count)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T, Port, Buffer> core::fmt::Debug for SyncRead<'_, T, Port, Buffer>
where
	T: Data,
	Port: SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SyncRead")
			.field("serial_port", self.client.serial_port())
			.field("motor_ids", &self.motor_ids)
			.field("count", &T::ENCODED_SIZE)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

// The read methods are bisync: on the async client they are `async fn`s; on the sync client the
// `Iterator` impl below drives them. `remaining`/`pop_motor_id` carry no bus I/O, so they are
// emitted unchanged in both flavours.
#[super::bisync]
impl<T, Port, Buffer> SyncReadBytes<'_, T, Port, Buffer>
where
	T: ?Sized,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	/// Read the next motor reply, or [`None`] once every motor has replied.
	pub async fn read_next<'a>(&'a mut self) -> Option<Result<Response<T>, ReadError<Port::Error>>>
	where
		T: From<&'a [u8]>,
	{
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response(motor_id).await)
	}

	/// Read the next motor reply borrowing the data from the internal read buffer, or [`None`] once every motor has replied.
	pub async fn read_next_borrow(&mut self) -> Option<Result<Response<&T>, ReadError<Port::Error>>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response_borrow(motor_id).await)
	}

	async fn next_response<'a>(&'a mut self, motor_id: u8) -> Result<Response<T>, ReadError<Port::Error>>
	where
		T: From<&'a [u8]>,
	{
		let response = self.client.read_status_response(self.count, true).await?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
		Ok(decode_status_packet_bytes(response)?)
	}

	async fn next_response_borrow(&mut self, motor_id: u8) -> Result<Response<&T>, ReadError<Port::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let response = self.client.read_status_response(self.count, true).await?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
		Ok(decode_status_packet_bytes_borrow(response)?)
	}
}

#[super::bisync]
impl<T, Port, Buffer> SyncRead<'_, T, Port, Buffer>
where
	T: Data,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	/// Read the next motor reply, or [`None`] once every motor has replied.
	pub async fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<Port::Error>>> {
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response(motor_id).await)
	}

	async fn next_response(&mut self, motor_id: u8) -> Result<Response<T>, ReadError<Port::Error>> {
		let response = self.client.read_status_response(T::ENCODED_SIZE, true).await?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), T::ENCODED_SIZE.into())?;
		decode_status_packet(response)
	}
}

// `Iterator` and `Drop` are synchronous-only: `Iterator::next` cannot `.await`, and `Drop` cannot
// drain the bus asynchronously. The async client uses `read_next().await` and relies on the next
// `write_instruction` discarding any unread responses.
#[super::only_sync]
impl<T, Port, Buffer> Iterator for SyncReadBytes<'_, T, Port, Buffer>
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
impl<T, Port, Buffer> Drop for SyncReadBytes<'_, T, Port, Buffer>
where
	T: ?Sized,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.pop_motor_id().is_some() {
			self.client.read_status_response(self.count, true).ok();
		}
	}
}

#[super::only_sync]
impl<T, Port, Buffer> Iterator for SyncRead<'_, T, Port, Buffer>
where
	T: Data,
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
impl<T, Port, Buffer> Drop for SyncRead<'_, T, Port, Buffer>
where
	T: Data,
	Port: SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.read_next().is_some() {}
	}
}
