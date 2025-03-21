use core::marker::PhantomData;

use crate::bus::endian::write_u16_le;
use crate::bus::data::{decode_status_packet, decode_status_packet_bytes, decode_status_packet_bytes_borrow};
use crate::bus::data::Data;
use crate::{Client, ReadError, Response, WriteError};
use super::{instruction_id, packet_id};

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read a number of bytes from multiple motors in one command.
	pub fn sync_read_bytes<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<SyncReadBytes<'a, T, SerialPort, Buffer>, WriteError<SerialPort::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})?;

		Ok(SyncReadBytes {
			client: self,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}

	/// Synchronously read a number of bytes from multiple motors in one command.
	pub fn sync_read_bytes_borrow<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<SyncReadBytes<'a, T, SerialPort, Buffer>, WriteError<SerialPort::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})?;

		Ok(SyncReadBytes {
			client: self,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}

	/// Synchronously read values from multiple motors in one command.
	pub fn sync_read<'a, T: Data>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<SyncRead<'a, T, SerialPort, Buffer>, WriteError<SerialPort::Error>> {
		let count = T::ENCODED_SIZE;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
			Ok(())
		})?;

		Ok(SyncRead {
			client: self,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}
}

macro_rules! make_sync_read_bytes_struct {
	($($DefaultSerialPort:ty)?) => {
		/// A sync read operation that returns unparsed bytes.
		pub struct SyncReadBytes<'a, T, SerialPort $(= $DefaultSerialPort)?, Buffer = crate::bus::DefaultBuffer>
		where
			T: ?Sized,
			SerialPort: crate::SerialPort,
			Buffer: AsRef<[u8]> + AsMut<[u8]>,
		{
			client: &'a mut Client<SerialPort, Buffer>,
			count: u16,
			motor_ids: &'a [u8],
			index: usize,
			data: PhantomData<fn() -> T>,
		}
	}
}

#[cfg(feature = "serial2")]
make_sync_read_bytes_struct!(serial2::SerialPort);

#[cfg(not(feature = "serial2"))]
make_sync_read_bytes_struct!();

impl<T, SerialPort, Buffer> core::fmt::Debug for SyncReadBytes<'_, T, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort + core::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("SyncRead")
			.field("serial_port", self.client.serial_port())
			.field("motor_ids", &self.motor_ids)
			.field("count", &self.count)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T, SerialPort, Buffer> Drop for SyncReadBytes<'_, T, SerialPort, Buffer>
where
	T: ?Sized,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.pop_motor_id().is_some() {
			self.client.read_status_response(self.count).ok();
		}
	}
}

impl<T, SerialPort, Buffer> Iterator for SyncReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<T>, crate::error::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

impl<T, SerialPort, Buffer> SyncReadBytes<'_, T, SerialPort, Buffer>
where
	T: ?Sized,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	/// Read the next motor reply.
	pub fn read_next<'a>(&'a mut self) -> Option<Result<Response<T>, ReadError<SerialPort::Error>>>
	where
		T: From<&'a [u8]>,
	{
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response(motor_id))
	}

	/// Read the next motor reply, borrowing the data from the internal read buffer.
	pub fn read_next_borrow<'a>(&'a mut self) -> Option<Result<Response<&'a T>, ReadError<SerialPort::Error>>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response_borrow(motor_id))
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	fn next_response<'a>(&'a mut self, motor_id: u8) -> Result<Response<T>, ReadError<SerialPort::Error>>
	where
		T: From<&'a [u8]>,
	{
		let response = self.client.read_status_response(self.count)?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
		Ok(decode_status_packet_bytes(response)?)
	}

	fn next_response_borrow<'a>(&'a mut self, motor_id: u8) -> Result<Response<&'a T>, ReadError<SerialPort::Error>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		let response = self.client.read_status_response(self.count)?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
		Ok(decode_status_packet_bytes_borrow(response)?)
	}
}
macro_rules! make_sync_read_struct {
	($($DefaultSerialPort:ty)?) => {
		/// A sync read operation that returns parsed values.
		pub struct SyncRead<'a, T, SerialPort $(= $DefaultSerialPort)?, Buffer = crate::bus::DefaultBuffer>
		where
			T: Data,
			SerialPort: crate::SerialPort,
			Buffer: AsRef<[u8]> + AsMut<[u8]>,
		{
			client: &'a mut Client<SerialPort, Buffer>,
			motor_ids: &'a [u8],
			index: usize,
			data: PhantomData<fn() -> T>,
		}
	}
}

#[cfg(feature = "serial2")]
make_sync_read_struct!(serial2::SerialPort);

#[cfg(not(feature = "serial2"))]
make_sync_read_struct!();

impl<T, SerialPort, Buffer> core::fmt::Debug for SyncRead<'_, T, SerialPort, Buffer>
where
	T: Data,
	SerialPort: crate::SerialPort + core::fmt::Debug,
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

impl<T, SerialPort, Buffer> Drop for SyncRead<'_, T, SerialPort, Buffer>
where
	T: Data,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.read_next().is_some() {}
	}
}

impl<T, SerialPort, Buffer> Iterator for SyncRead<'_, T, SerialPort, Buffer>
where
	T: Data,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<T>, crate::error::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

impl<T, SerialPort, Buffer> SyncRead<'_, T, SerialPort, Buffer>
where
	T: Data,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	/// Read the next motor reply.
	pub fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<SerialPort::Error>>>
	where
		T: Data,
	{
		let motor_id = self.pop_motor_id()?;
		Some(self.next_response(motor_id))
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	fn next_response(&mut self, motor_id: u8) -> Result<Response<T>, ReadError<SerialPort::Error>> {
		let response = self.client.read_status_response(T::ENCODED_SIZE)?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), T::ENCODED_SIZE.into())?;

		Ok(decode_status_packet(response)?)
	}
}
