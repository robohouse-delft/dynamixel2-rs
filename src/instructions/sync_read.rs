use super::{instruction_id, packet_id, SyncData};
use crate::endian::{read_u8_le, read_u16_le, read_u32_le, write_u16_le};
use crate::{Bus, ReadError, WriteError, TransferError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read an arbitrary number of bytes from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read_cb<'a, F>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
		mut on_response: F,
	) -> Result<(), WriteError>
	where
		F: FnMut(Result<SyncData<&[u8]>, ReadError>),
	{
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			buffer[4..].copy_from_slice(motor_ids);
		})?;
		for &motor_id in motor_ids {
			let response = self.read_status_response().and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), count.into())?;
				Ok(response)
			});

			match response {
				Ok(response) => on_response(Ok(SyncData {
					motor_id,
					data: response.parameters(),
				})),
				Err(e) => on_response(Err(e)),
			}
		}
		Ok(())
	}

	/// Synchronously read an 8 bit value from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read_u8_cb<'a, F>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		mut on_response: F,
	) -> Result<(), WriteError>
	where
		F: FnMut(Result<SyncData<u8>, ReadError>),
	{
		let count = 1;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			buffer[4..].copy_from_slice(motor_ids);
		})?;
		for &motor_id in motor_ids {
			let data = self.read_status_response().and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), count)?;
				Ok(SyncData {
					motor_id,
					data: read_u8_le(response.parameters()),
				})
			});
			on_response(data);
		}
		Ok(())
	}

	/// Synchronously read a 16 bit value from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read_u16_cb<'a, F>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		mut on_response: F,
	) -> Result<(), WriteError>
	where
		F: FnMut(Result<SyncData<u16>, ReadError>),
	{
		let count = 1;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			buffer[4..].copy_from_slice(motor_ids);
		})?;
		for &motor_id in motor_ids {
			let data = self.read_status_response().and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), count)?;
				Ok(SyncData {
					motor_id,
					data: read_u16_le(response.parameters()),
				})
			});
			on_response(data);
		}
		Ok(())
	}

	/// Synchronously read a 32 bit value from multiple motors in one command.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	pub fn sync_read_u32_cb<'a, F>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		mut on_response: F,
	) -> Result<(), WriteError>
	where
		F: FnMut(Result<SyncData<u32>, ReadError>),
	{
		let count = 4;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 4 + motor_ids.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			buffer[4..].copy_from_slice(motor_ids);
		})?;
		for &motor_id in motor_ids {
			let data = self.read_status_response().and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), count)?;
				Ok(SyncData {
					motor_id,
					data: read_u32_le(response.parameters()),
				})
			});
			on_response(data);
		}
		Ok(())
	}

	/// Synchronously read an arbitrary number of bytes from multiple motors in one command.
	///
	/// If this function fails to get the data from any of the motors, the entire function retrns an error.
	/// If you need access to the data from other motors, or if you want acces to the error for each motor, see [`Self::sync_read_cb`].
	pub fn sync_read<'a>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<Vec<SyncData<Vec<u8>>>, TransferError> {
		let mut result = Vec::with_capacity(motor_ids.len());
		let mut read_error = None;
		self.sync_read_cb(motor_ids, address, count, |data| {
			match data {
				Err(e) if read_error.is_none() => read_error = Some(e),
				Err(_) => (),
				Ok(response) => result.push(SyncData {
					motor_id: response.motor_id,
					data: response.data.to_owned(),
				}),
			}
		})?;
		Ok(result)
	}

	/// Synchronously read an 8 bit value from multiple motors in one command.
	///
	/// If this function fails to get the data from any of the motors, the entire function retrns an error.
	/// If you need access to the data from other motors, or if you want acces to the error for each motor, see [`Self::sync_read_u8_cb`].
	pub fn sync_read_u8<'a>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<Vec<SyncData<u8>>, TransferError> {
		let mut result = Vec::with_capacity(motor_ids.len());
		let mut read_error = None;
		self.sync_read_u8_cb(motor_ids, address, |data| {
			match data {
				Err(e) if read_error.is_none() => read_error = Some(e),
				Err(_) => (),
				Ok(data) => result.push(data),
			}
		})?;
		Ok(result)
	}

	/// Synchronously read a 16 bit value from multiple motors in one command.
	///
	/// If this function fails to get the data from any of the motors, the entire function retrns an error.
	/// If you need access to the data from other motors, or if you want acces to the error for each motor, see [`Self::sync_read_u16_cb`].
	pub fn sync_read_u16<'a>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<Vec<SyncData<u16>>, TransferError> {
		let mut result = Vec::with_capacity(motor_ids.len());
		let mut read_error = None;
		self.sync_read_u16_cb(motor_ids, address, |data| {
			match data {
				Err(e) if read_error.is_none() => read_error = Some(e),
				Err(_) => (),
				Ok(data) => result.push(data),
			}
		})?;
		Ok(result)
	}

	/// Synchronously read a 32 bit value from multiple motors in one command.
	///
	/// If this function fails to get the data from any of the motors, the entire function retrns an error.
	/// If you need access to the data from other motors, or if you want acces to the error for each motor, see [`Self::sync_read_u32_cb`].
	pub fn sync_read_u32<'a>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<Vec<SyncData<u32>>, TransferError> {
		let mut result = Vec::with_capacity(motor_ids.len());
		let mut read_error = None;
		self.sync_read_u32_cb(motor_ids, address, |data| {
			match data {
				Err(e) if read_error.is_none() => read_error = Some(e),
				Err(_) => (),
				Ok(data) => result.push(data),
			}
		})?;
		Ok(result)
	}
}
