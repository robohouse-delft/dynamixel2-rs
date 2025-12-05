use super::Client;
use crate::bus::endian::write_u16_le;
use crate::bus::instruction_id;
use crate::bus::Data;
use crate::{Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Register a write of an arbitrary number of bytes, to be triggered later by an `action` command.
	///
	/// Only one write command can be registered per motor.
	///
	/// You can have all connected motors execute their registered write using [`Self::broadcast_action`],
	/// or a single motor using [`Self::action`].
	///
	/// You may specify [`crate::bus::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub async fn reg_write_bytes(
		&mut self,
		motor_id: u8,
		address: u16,
		data: &[u8],
	) -> Result<Response<()>, TransferError<SerialPort::Error>> {
		self.write_instruction(motor_id, instruction_id::REG_WRITE, 2 + data.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2..].copy_from_slice(data);
			Ok(())
		})
		.await?;
		Ok(self.read_response_if_not_broadcast(motor_id).await?)
	}

	/// Register a write command for value to a specific motor.
	///
	/// Only one write command can be registered per motor.
	///
	/// You can have all connected motors execute their registered write using [`Self::broadcast_action`],
	/// or a single motor using [`Self::action`].
	///
	/// You may specify [`crate::bus::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	pub async fn reg_write<T: Data>(
		&mut self,
		motor_id: u8,
		address: u16,
		value: &T,
	) -> Result<Response<()>, TransferError<SerialPort::Error>> {
		self.write_instruction(motor_id, instruction_id::REG_WRITE, 2 + 1, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			value.encode(&mut buffer[2..])
		})
		.await?;
		Ok(self.read_response_if_not_broadcast(motor_id).await?)
	}
}
