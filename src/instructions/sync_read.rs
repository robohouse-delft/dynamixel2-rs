use core::marker::PhantomData;

use crate::bus::endian::write_u16_le;
use crate::bus::{Data, StatusPacket};
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
		})?;

		Ok(SyncRead {
			client: self,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}
}

/// A sync read operation that returns unparsed bytes.
pub struct SyncReadBytes<'a, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<SerialPort, Buffer>,
	count: u16,
	motor_ids: &'a [u8],
	index: usize,
	data: PhantomData<fn() -> T>,
}

impl<T, SerialPort, Buffer> core::fmt::Debug for SyncReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort + std::fmt::Debug,
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
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next().is_some() {}
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
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	/// Read the next motor reply.
	pub fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<SerialPort::Error>>> {
		let motor_id = self.pop_motor_id()?;
		let response: Response<&[u8]> = match self.next_status_packet(motor_id) {
			Ok(status) => status.into(),
			Err(e) => return Some(Err(e)),
		};
		Some(Ok(Response {
			motor_id: response.motor_id,
			alert: response.alert,
			data: T::from(response.data),
		}))
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	fn next_status_packet(&mut self, motor_id: u8) -> Result<StatusPacket, ReadError<SerialPort::Error>> {
		self.client.read_status_response(self.count).and_then(|response| {
			// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
			// We need to report a timeout or something for the missed motor though.
			crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
			crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
			Ok(response)
		})
	}
}

/// A sync read operation that returns parsed values.
pub struct SyncRead<'a, T, SerialPort, Buffer>
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

impl<T, SerialPort, Buffer> core::fmt::Debug for SyncRead<'_, T, SerialPort, Buffer>
where
	T: Data,
	SerialPort: crate::SerialPort + std::fmt::Debug,
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
		let response: Response<&[u8]> = match self.next_status_packet(motor_id) {
			Ok(status) => status.into(),
			Err(e) => return Some(Err(e)),
		};
		let data = match T::decode(response.data) {
			Ok(x) => x,
			Err(e) => return Some(Err(e.into())),
		};
		Some(Ok(Response {
			motor_id: response.motor_id,
			alert: response.alert,
			data,
		}))
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	fn next_status_packet(&mut self, motor_id: u8) -> Result<StatusPacket, ReadError<SerialPort::Error>> {
		self.client.read_status_response(T::ENCODED_SIZE).and_then(|response| {
			// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
			// We need to report a timeout or something for the missed motor though.
			crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
			crate::InvalidParameterCount::check(response.parameters().len(), T::ENCODED_SIZE.into())?;
			Ok(response)
		})
	}
}
