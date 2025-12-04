use super::Client;
use crate::bus::data::{decode_status_packet, decode_status_packet_bytes};
use crate::bus::endian::write_u16_le;
use crate::bus::{Data, StatusPacket};
use crate::instruction_id;
use crate::{Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	async fn read_raw(&mut self, motor_id: u8, address: u16, count: u16) -> Result<StatusPacket<'_>, TransferError<SerialPort::Error>> {
		let response = self
			.transfer_single(motor_id, instruction_id::READ, 4, count, |buffer| {
				write_u16_le(&mut buffer[0..], address);
				write_u16_le(&mut buffer[2..], count);
				Ok(())
			})
			.await?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), count.into()).map_err(crate::ReadError::from)?;
		Ok(response)
	}

	/// Read an arbitrary number of bytes from a specific motor.
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub async fn read_bytes<'a, T>(
		&'a mut self,
		motor_id: u8,
		address: u16,
		count: u16,
	) -> Result<Response<T>, TransferError<SerialPort::Error>>
	where
		T: From<&'a [u8]>,
	{
		let status = self.read_raw(motor_id, address, count).await?;
		Ok(decode_status_packet_bytes(status)?)
	}

	/// Read a value from a specific motor.
	///
	/// Specify the return type using turbofish: `client.read::<u8>`
	///
	/// This function will not work correctly if the motor ID is set to [`packet_id::BROADCAST`][crate::instructions::packet_id::BROADCAST].
	/// Use [`Self::sync_read`] to read from multiple motors with one command.
	pub async fn read<T>(&mut self, motor_id: u8, address: u16) -> Result<Response<T>, TransferError<SerialPort::Error>>
	where
		T: Data,
	{
		let status = self.read_raw(motor_id, address, T::ENCODED_SIZE).await?;
		Ok(decode_status_packet(status)?)
	}
}
