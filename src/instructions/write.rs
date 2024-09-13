use super::{instruction_id, read_response_if_not_broadcast};
use crate::endian::{write_u16_le, write_u32_le};
use crate::transport::Transport;
use crate::{Bus, Response, TransferError};

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: Transport,
{
	/// Write an arbitrary number of bytes to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write(&mut self, motor_id: u8, address: u16, data: &[u8]) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + data.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2..].copy_from_slice(data)
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Write an 8 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u8(&mut self, motor_id: u8, address: u16, value: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + 1, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2] = value;
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Write an 16 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u16(&mut self, motor_id: u8, address: u16, value: u16) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + 2, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], value);
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Write an 32 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u32(&mut self, motor_id: u8, address: u16, value: u32) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + 4, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u32_le(&mut buffer[2..], value);
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}
}
