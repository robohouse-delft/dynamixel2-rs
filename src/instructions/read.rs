use super::instruction_id;
use crate::endian::{read_u16_le, read_u32_le, write_u16_le};
use crate::{Bus, TransferError};

pub struct ReadResponse<'a, Stream, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	pub(crate) response: crate::Response<'a, Stream, ReadBuffer, WriteBuffer>,
}

impl<'a, Stream, ReadBuffer, WriteBuffer> ReadResponse<'a, Stream, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the ID of the motor.
	pub fn motor_id(&self) -> u8 {
		self.response.packet_id()
	}

	/// Get the read data as byte slice.
	///
	/// The individual registers of the motor are encoded as little-endian.
	/// Refer to the online manual of your motor for the addresses and sizes of all registers.
	pub fn data(&self) -> &[u8] {
		self.response.parameters()
	}
}

impl<Stream, ReadBuffer, WriteBuffer> Bus<Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read an arbitrary number of bytes from a specific motor.
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub fn read(&mut self, motor_id: u8, address: u16, count: u16) -> Result<ReadResponse<Stream, ReadBuffer, WriteBuffer>, TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::READ, 4, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
		})?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), count.into()).map_err(crate::ReadError::from)?;
		Ok(ReadResponse { response })
	}

	/// Read an 8 bit register from a specific motor.
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub fn read_u8(&mut self, motor_id: u8, address: u16) -> Result<u8, TransferError> {
		let response = self.read(motor_id, address, 1)?;
		Ok(response.data()[0])
	}

	/// Read 16 bit register from a specific motor.
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub fn read_u16(&mut self, motor_id: u8, address: u16) -> Result<u16, TransferError> {
		let response = self.read(motor_id, address, 2)?;
		Ok(read_u16_le(response.data()))
	}

	/// Read 32 bit register from a specific motor.
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub fn read_u32(&mut self, motor_id: u8, address: u16) -> Result<u32, TransferError> {
		let response = self.read(motor_id, address, 4)?;
		Ok(read_u32_le(response.data()))
	}
}
