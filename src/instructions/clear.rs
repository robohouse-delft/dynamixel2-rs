use super::{instruction_id, packet_id};
use crate::serial_port::SerialPort;
use crate::{Client, Response, TransferError, WriteError};

/// The parameters for the CLEAR command to clear the revolution counter.
const CLEAR_REVOLUTION_COUNT: [u8; 5] = [0x01, 0x44, 0x58, 0x4C, 0x22];

/// The parameters for the CLEAR command to clear the error state.
///
/// This is only supported on some motors.
const CLEAR_ERROR: [u8; 5] = [0x01, 0x45, 0x52, 0x43, 0x4C];

impl<ReadBuffer, WriteBuffer, T> Client<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
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
		self.write_instruction(motor_id, instruction_id::CLEAR, CLEAR_REVOLUTION_COUNT.len(), clear_revolution_count_parameters)?;
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
			clear_revolution_count_parameters,
		)?;
		Ok(())
	}

	/// Clear the error of a motor.
	///
	/// This will reset the "error code" register to 0 if the error can be cleared.
	/// If the error cannot be cleared, the function returns a [`MotorError`](crate::MotorError) with error code `0x01`.
	///
	/// This instruction is currently only implemented on the Dynamixel Y series.
	pub fn clear_error(&mut self, motor_id: u8) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::CLEAR, CLEAR_ERROR.len(), clear_error_parameters)?;
		Ok(super::read_response_if_not_broadcast(self, motor_id)?)

	}

	/// Try to clear the error of all motors on the bus.
	///
	/// This will reset the "error code" register to 0 if the error can be cleared
	/// and if the instruction is supported by the motor.
    ///
	/// This instruction is currently only implemented on the Dynamixel Y series.
	pub fn broadcast_clear_error(&mut self) -> Result<(), WriteError<T::Error>> {
		self.write_instruction(
			packet_id::BROADCAST,
			instruction_id::CLEAR,
			CLEAR_ERROR.len(),
			clear_error_parameters,
		)?;
		Ok(())
	}
}

fn clear_revolution_count_parameters(buffer: &mut [u8]) {
	buffer.copy_from_slice(&CLEAR_REVOLUTION_COUNT)
}

fn clear_error_parameters(buffer: &mut [u8]) {
	buffer.copy_from_slice(&CLEAR_ERROR)
}
