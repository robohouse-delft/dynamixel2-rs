use super::{instruction_id, packet_id};
use crate::{Bus, TransferError, WriteError};

impl<Stream, ReadBuffer, WriteBuffer> Bus<Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Send a reboot command to a specific motor.
	///
	/// Certain error conditions can only be cleared by rebooting a motor.
	/// When a motor reboots, all volatile (non-EEPROM) registers are reset to their initial value.
	/// This also has the effect of disabling motor torque and resetting the multi-revolution information.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_reboot`] is generally easier to use.
	pub fn reboot(&mut self, motor_id: u8) -> Result<(), TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_action()?;
		} else {
			let response = self.transfer_single(motor_id, instruction_id::REBOOT, 0, |_| ())?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
	}

	/// Broadcast an reboot command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_reboot(&mut self) -> Result<(), WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::REBOOT, 0, |_| ())
	}
}
