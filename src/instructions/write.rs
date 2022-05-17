use super::instruction_id;
use crate::endian::{write_u16_le, write_u32_le};
use crate::{Bus, TransferError};

#[cfg(feature = "sync")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Write an arbitrary number of bytes to a specific motor.
	pub fn write(&mut self, motor_id: u8, address: u16, data: &[u8]) -> Result<(), TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::WRITE, 2 + data.len(), |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2..].copy_from_slice(data)
		})?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 8 bit value to a specific motor.
	pub fn write_u8(&mut self, motor_id: u8, address: u16, value: u8) -> Result<(), TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::WRITE, 2 + 1, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			buffer[2] = value;
		})?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 16 bit value to a specific motor.
	pub fn write_u16(&mut self, motor_id: u8, address: u16, value: u16) -> Result<(), TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::WRITE, 2 + 2, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], value);
		})?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 32 bit value to a specific motor.
	pub fn write_u32(&mut self, motor_id: u8, address: u16, value: u32) -> Result<(), TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::WRITE, 2 + 4, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u32_le(&mut buffer[2..], value);
		})?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}
}

#[cfg(any(feature = "async_smol", feature = "async_tokio"))]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Write an arbitrary number of bytes to a specific motor.
	pub async fn write(&mut self, motor_id: u8, address: u16, data: &[u8]) -> Result<(), TransferError> {
		let response = self
			.transfer_single(motor_id, instruction_id::WRITE, 2 + data.len(), |buffer| {
				write_u16_le(&mut buffer[0..], address);
				buffer[2..].copy_from_slice(data)
			})
			.await?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 8 bit value to a specific motor.
	pub async fn write_u8(&mut self, motor_id: u8, address: u16, value: u8) -> Result<(), TransferError> {
		let response = self
			.transfer_single(motor_id, instruction_id::WRITE, 2 + 1, |buffer| {
				write_u16_le(&mut buffer[0..], address);
				buffer[2] = value;
			})
			.await?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 16 bit value to a specific motor.
	pub async fn write_u16(&mut self, motor_id: u8, address: u16, value: u16) -> Result<(), TransferError> {
		let response = self
			.transfer_single(motor_id, instruction_id::WRITE, 2 + 2, |buffer| {
				write_u16_le(&mut buffer[0..], address);
				write_u16_le(&mut buffer[2..], value);
			})
			.await?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}

	/// Write an 32 bit value to a specific motor.
	pub async fn write_u32(&mut self, motor_id: u8, address: u16, value: u32) -> Result<(), TransferError> {
		let response = self
			.transfer_single(motor_id, instruction_id::WRITE, 2 + 4, |buffer| {
				write_u16_le(&mut buffer[0..], address);
				write_u32_le(&mut buffer[2..], value);
			})
			.await?;
		crate::error::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		Ok(())
	}
}
