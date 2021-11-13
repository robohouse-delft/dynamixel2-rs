use super::{instruction_id, packet_id};
use crate::Bus;

/// The parameters for the CLEAR command to clear the revolution counter.
const CLEAR_REVOLUTION_COUNT: [u8; 5] = [0x01, 0x44, 0x58, 0x4C, 0x22];

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Clear the multi-revolution counter of a motor.
	///
	/// This will reset the "present position" register to a value between 0 and a whole revolution.
	/// It is not possible to clear the revolution counter of a motor while it is moving.
	/// Doing so will cause the motor to return an error, and the revolution counter will not be reset.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_clear_revolution_counter`] is generally easier to use.
	pub fn clear_revolution_counter(&mut self, motor_id: u8) -> Result<(), crate::TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_clear_revolution_counter()?;
		} else {
			let response = self.transfer_single(motor_id, instruction_id::CLEAR, CLEAR_REVOLUTION_COUNT.len(), encode_parameters)?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
	}

	/// Clear the revolution counter of all connected motors.
	///
	/// This will reset the "present position" register to a value between 0 and a whole revolution.
	/// It is not possible to clear the mutli-revolution counter of a motor while it is moving.
	/// Doing so will cause the motor to return an error, and the revolution counter will not be reset.
	pub fn broadcast_clear_revolution_counter(&mut self) -> Result<(), crate::WriteError> {
		self.write_instruction(
			packet_id::BROADCAST,
			instruction_id::CLEAR,
			CLEAR_REVOLUTION_COUNT.len(),
			encode_parameters,
		)?;
		Ok(())
	}
}

fn encode_parameters(buffer: &mut [u8]) {
	buffer.copy_from_slice(&CLEAR_REVOLUTION_COUNT)
}
