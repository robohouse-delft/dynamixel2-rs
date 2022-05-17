use super::{instruction_id, packet_id};
use crate::{Bus, TransferError, WriteError};

#[cfg(feature = "sync")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Send an action command to trigger a previously registered instruction.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_action`] is generally easier to use.
	pub fn action(&mut self, motor_id: u8) -> Result<(), TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_action()?;
		} else {
			let response = self.transfer_single(motor_id, instruction_id::ACTION, 0, |_| ())?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
	}

	/// Broadcast an action command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_action(&mut self) -> Result<(), WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::ACTION, 0, |_| ())
	}
}

#[cfg(any(feature = "async_smol", feature = "async_tokio"))]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Send an action command to trigger a previously registered instruction.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_action`] is generally easier to use.
	pub async fn action(&mut self, motor_id: u8) -> Result<(), TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_action().await?;
		} else {
			let response = self.transfer_single(motor_id, instruction_id::ACTION, 0, |_| ()).await?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
	}

	/// Broadcast an action command to all connected motors to trigger a previously registered instruction.
	pub async fn broadcast_action(&mut self) -> Result<(), WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::ACTION, 0, |_| ())
			.await
	}
}
