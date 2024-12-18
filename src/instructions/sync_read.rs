use super::{instruction_id, packet_id};
use crate::endian::write_u16_le;
use crate::serial_port::SerialPort;
use crate::{Bus, ReadError, Response, StatusPacket, WriteError};

use crate::packet::Packet;
use std::marker::PhantomData;
use crate::packet::Read;

pub struct SyncRead<'a, ReadBuffer, WriteBuffer, T, Data>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
	Data: Read
{
	bus: &'a mut Bus<ReadBuffer, WriteBuffer, T>,
	count: u16,
	motor_ids: &'a [u8],
	phantom_data: PhantomData<Data>
}

impl<'a, ReadBuffer, WriteBuffer, Serial, Data> SyncRead<'a, ReadBuffer, WriteBuffer, Serial, Data>
	where
		ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
		WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
		Serial: SerialPort,
		Data: Read
{
	fn pop_motor_id(&mut self) -> Option<u8> {
		let (motor_id, remaining)  = self.motor_ids.split_first()?;
		self.motor_ids = remaining;
		Some(*motor_id)
	}
	fn status(&mut self, motor_id: u8) -> Result<StatusPacket, ReadError<Serial::Error>> {
		self.bus.read_status_response(self.count).and_then(|response| {
			crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
			crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
			Ok(response)
		})
	}
	pub fn next_raw(&'a mut self) -> Option<Result<Response<&'a [u8]>, ReadError<Serial::Error>>>{
		let motor_id = self.pop_motor_id()?;
		let response = self.status(motor_id);
		Some(response.map(Into::into))
	}

	pub fn next(&mut self) -> Option<Result<Response<Data>, ReadError<Serial::Error>>>{
		let motor_id = self.pop_motor_id()?;
		let response = self.status(motor_id);
		let response = match response {
			Ok(response) => response.try_into_response().map_err(Into::into),
			Err(e) => Err(e),
		};
		Some(response)
	}
}

impl<ReadBuffer, WriteBuffer, Serial, Data> Iterator for SyncRead<'_, ReadBuffer, WriteBuffer, Serial, Data>
where
ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
Serial: SerialPort,
Data: Read
{
	type Item = Result<Response<Data>, ReadError<Serial::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next()
	}
}

impl<ReadBuffer, WriteBuffer, T, Data> Drop for SyncRead<'_, ReadBuffer, WriteBuffer, T, Data>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
	Data: Read
{
	fn drop(&mut self) {
		while self.next().is_some() {}
	}
}

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Synchronously read an arbitrary number of bytes from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read<'a, Data>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<SyncRead<'a, ReadBuffer, WriteBuffer, T, Data>, WriteError<T::Error>>
	where Data: Read
	{
		let count = Data::R_COUNT;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
		})?;

		Ok(SyncRead {
			bus: self,
			count,
			motor_ids,
			phantom_data: PhantomData
		})
	}

	/// Synchronously read an 8 bit value from multiple motors in one command.
	///
	/// Return a `SyncRead` which can be iterated over to collect the data
	pub fn sync_read_u8<'a>(&'a mut self, motor_ids: &'a [u8], address: u16) -> Result<SyncRead<'a, ReadBuffer, WriteBuffer, T, u8>, WriteError<T::Error>> {
		self.sync_read(motor_ids, address)
	}
	/// Synchronously read an 8 bit value from multiple motors in one command.
	///
	/// Return a `SyncRead` which can be iterated over to collect the data
	pub fn sync_read_u16<'a>(&'a mut self, motor_ids: &'a [u8], address: u16) -> Result<SyncRead<'a, ReadBuffer, WriteBuffer, T, u16>, WriteError<T::Error>> {
		self.sync_read(motor_ids, address)
	}
	/// Synchronously read an 16 bit value from multiple motors in one command.
	///
	/// Return a `SyncRead` which can be iterated over to collect the data
	pub fn sync_read_u32<'a>(&'a mut self, motor_ids: &'a [u8], address: u16) -> Result<SyncRead<'a, ReadBuffer, WriteBuffer, T, u32>, WriteError<T::Error>> {
		self.sync_read(motor_ids, address)
	}
}
