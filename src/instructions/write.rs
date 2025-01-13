use crate::bus::endian::write_u16_le;
use crate::{Client, Response, TransferError};
use crate::bus::Data;
use super::{instruction_id, read_response_if_not_broadcast};

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Write an arbitrary number of bytes to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write_bytes(&mut self, motor_id: u8, address: u16, data: &[u8]) -> Result<Response<()>, TransferError<SerialPort::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + data.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2..].copy_from_slice(data);
			Ok(())
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Write value to a specific motor.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub fn write<T: Data>(&mut self, motor_id: u8, address: u16, data: &T) -> Result<Response<()>, TransferError<SerialPort::Error>> {
		self.write_instruction(motor_id, instruction_id::WRITE, 2 + T::ENCODED_SIZE as usize, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			data.encode(&mut buffer[2..])?;
			Ok(())
		})?;
		Ok(read_response_if_not_broadcast(self, motor_id)?)
	}
}
