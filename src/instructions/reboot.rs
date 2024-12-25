use crate::serial_port::SerialPort;
use crate::{Client, Response, TransferError, WriteError};
use super::{instruction_id, packet_id};

impl<ReadBuffer, WriteBuffer, T> Client<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Send a reboot command to a specific motor.
	///
	/// Certain error conditions can only be cleared by rebooting a motor.
	/// When a motor reboots, all volatile (non-EEPROM) registers are reset to their initial value.
	/// This also has the effect of disabling motor torque and resetting the multi-revolution information.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	///
	/// If you want to broadcast this instruction, it may be more convenient to use [`Self::broadcast_reboot()`] instead.
	pub fn reboot(&mut self, motor_id: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::REBOOT, 0, |_| ())?;
		Ok(super::read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Broadcast an reboot command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_reboot(&mut self) -> Result<(), WriteError<T::Error>> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::REBOOT, 0, |_| ())
	}
}
