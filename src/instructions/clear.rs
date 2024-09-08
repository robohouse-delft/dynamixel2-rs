use super::{instruction_id, packet_id};
use crate::systems::{System, SerialPort};
use crate::{Bus, Response, TransferError, WriteError};

/// The parameters for the CLEAR command to clear the revolution counter.
const CLEAR_REVOLUTION_COUNT: [u8; 5] = [0x01, 0x44, 0x58, 0x4C, 0x22];

impl<ReadBuffer, WriteBuffer, S, T> Bus<ReadBuffer, WriteBuffer, S>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	S: System<Transport = T>,
	T: SerialPort,
{
	/// Clear the multi-revolution counter of a motor.
	///
	/// This will reset the "present position" register to a value between 0 and a whole revolution.
	/// It is not possible to clear the revolution counter of a motor while it is moving.
	/// Doing so will cause the motor to return an error, and the revolution counter will not be reset.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	///
	/// If you want to broadcast this instruction, it may be more convenient to use [`Self::broadcast_clear_revolution_counter()`] instead.
	pub fn clear_revolution_counter(&mut self, motor_id: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::CLEAR, CLEAR_REVOLUTION_COUNT.len(), encode_parameters)?;
		Ok(super::read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Clear the revolution counter of all connected motors.
	///
	/// This will reset the "present position" register to a value between 0 and a whole revolution.
	/// It is not possible to clear the mutli-revolution counter of a motor while it is moving.
	/// Doing so will cause the motor to return an error, and the revolution counter will not be reset.
	pub fn broadcast_clear_revolution_counter(&mut self) -> Result<(), WriteError<T::Error>> {
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
