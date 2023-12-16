use super::{instruction_id, packet_id};
use crate::{Bus, Response, TransferError, WriteError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Send an action command to trigger a previously registered instruction.
	///
	/// The `motor_id` parameter must not be set to [`packet_id::BROADCAST`],
	/// Instead use [`Self::broadcast_action`].
	pub fn action(&mut self, motor_id: u8) -> Result<Response<()>, TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::ACTION, 0, |_| ())?;
		Ok(response.try_into()?)
	}

	/// Broadcast an action command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_action(&mut self) -> Result<(), WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::ACTION, 0, |_| ())
	}
}
