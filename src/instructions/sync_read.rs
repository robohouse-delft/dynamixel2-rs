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
	/// Synchronously read an arbitrary number of bytes from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read<'a, T: Data<'a>>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<SyncRead<'a, T, SerialPort, Buffer>, WriteError<SerialPort::Error>> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
		})?;

		Ok(SyncRead {
			client: self,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}
}

/// A sync read operation.
///
/// Used to retrieve the responses of the different motors.
#[derive(Debug)]
pub struct SyncRead<'a, T, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<SerialPort, Buffer>,
	count: u16,
	motor_ids: &'a [u8],
	index: usize,
	data: PhantomData<fn() -> T>,
}

impl<'a, T, SerialPort, Buffer> SyncRead<'a, T, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read the next motor reply.
	pub fn next(&mut self) -> Option<Result<Response<T>, ReadError<SerialPort::Error>>>
	where
		T: Data<'a>,
	{
		let response = match self.next_response()? {
			Ok(x) => x,
			Err(e) => return Some(Err(e)),
		};
		let (data, used) = match T::decode(response.data) {
			Ok(x) => x,
			Err(e) => return Some(Err(e.into())),
		};
		if let Err(e) = crate::error::InvalidParameterCount::check(response.data.len(), used) {
			return Some(Err(e.into()));
		}
		Some(Ok(Response {
			motor_id: response.motor_id,
			alert: response.alert,
			data,
		}))
	}

	/// Read the next response without parsing the parameters.
	pub fn next_response(&mut self) -> Option<Result<Response<&[u8]>, ReadError<SerialPort::Error>>>{
		let motor_id = self.pop_motor_id()?;
		match self.next_status_packet(motor_id) {
			Ok(status) => Some(Ok(status.into())),
			Err(e) => Some(Err(e)),
		}
	}

	fn pop_motor_id(&mut self) -> Option<u8> {
		let motor_id = self.motor_ids.get(self.index)?;
		self.index += 1;
		Some(*motor_id)
	}

	fn next_status_packet(&mut self, motor_id: u8) -> Result<StatusPacket, ReadError<SerialPort::Error>> {
		self.client.read_status_response(self.count).and_then(|response| {
			// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
			// We need to report a timeout or somehing for the missed motor though.
			crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
			crate::InvalidParameterCount::check(response.parameters().len(), self.count.into())?;
			Ok(response)
		})
	}
}

impl<T, SerialPort, Buffer> Drop for SyncRead<'_, T, SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next_response().is_some() {}
	}
}

impl<'a, T, SerialPort, Buffer> Iterator for SyncRead<'a, T, SerialPort, Buffer>
where
	T: Data<'a> + 'static,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<T>, crate::error::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		Self::next(self)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.motor_ids.len() - self.index))
	}
}
