use super::{instruction_id, packet_id};
use crate::{Bus, Response, TransferError, WriteError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Send a reboot command to a specific motor.
	///
	/// Certain error conditions can only be cleared by rebooting a motor.
	/// When a motor reboots, all volatile (non-EEPROM) registers are reset to their initial value.
	/// This also has the effect of disabling motor torque and resetting the multi-revolution information.
	///
	/// The `motor_id` parameter must not be set to [`packet_id::BROADCAST`],
	/// Instead use [`Self::broadcast_reboot`].
	pub fn reboot(&mut self, motor_id: u8) -> Result<Response<()>, TransferError> {
		let response = self.transfer_single(motor_id, instruction_id::REBOOT, 0, |_| ())?;
		Ok(response.try_into()?)
	}

	/// Broadcast an reboot command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_reboot(&mut self) -> Result<(), WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::REBOOT, 0, |_| ())
	}
}
