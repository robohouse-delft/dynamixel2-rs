use super::{instruction_id, packet_id};
use crate::systems::{System, SerialPort};
use crate::{Bus, Response, TransferError, WriteError};

impl<ReadBuffer, WriteBuffer, S, T> Bus<ReadBuffer, WriteBuffer, S>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	S: System<Transport = T>,
	T: SerialPort,
{
	/// Send an action command to trigger a previously registered instruction.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	///
	/// If you want to broadcast this instruction, it may be more convenient to use [`Self::broadcast_action()`] instead.
	pub fn action(&mut self, motor_id: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::ACTION, 0, |_| ())?;
		Ok(super::read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Broadcast an action command to all connected motors to trigger a previously registered instruction.
	pub fn broadcast_action(&mut self) -> Result<(), WriteError<T::Error>> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::ACTION, 0, |_| ())
	}
}
