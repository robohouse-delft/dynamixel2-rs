use super::{instruction_id, read_response_if_not_broadcast};
use crate::endian::write_u16_le;
use crate::serial_port::SerialPort;
use crate::{Bus, Response, TransferError};
use crate::packet::Write;

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Write an arbitrary number of bytes to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write<Data: Write>(&mut self, motor_id: u8, address: u16, data: Data) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + Data::W_COUNT as usize, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			data.write_bytes(&mut buffer[2..]);
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Write an 8 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u8(&mut self, motor_id: u8, address: u16, value: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write(motor_id, address, value)
	}

	/// Write an 16 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u16(&mut self, motor_id: u8, address: u16, value: u16) -> Result<Response<()>, TransferError<T::Error>> {
		self.write(motor_id, address, value)
	}

	/// Write an 32 bit value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_u32(&mut self, motor_id: u8, address: u16, value: u32) -> Result<Response<()>, TransferError<T::Error>> {
		self.write(motor_id, address, value)
	}
}
