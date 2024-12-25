use super::{instruction_id, packet_id};
use crate::serial_port::SerialPort;
use crate::{Client, Response, TransferError, WriteError};

/// The kind of factory reset to perform.
#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FactoryResetKind {
	/// Reset all settings, including the motor ID and baud rate.
	ResetAll = 0xFF,

	/// Reset all settings except for the motor ID.
	KeepId = 0x01,

	/// Reset all settings except for the motor ID and baud rate.
	KeepIdAndBaudRate = 0x02,
}

impl<ReadBuffer, WriteBuffer, T> Client<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Reset the settings of a motor to the factory defaults.
	///
	/// This will reset all registers to the factory default, including the EEPROM registers.
	/// The only exceptions are the ID and baud rate settings, which may be kept depending on the `kind` argument.
	///
	/// You may specify [`crate::instructions::packet_id::BROADCAST`] as motor ID.
	/// If you do, none of the devices will reply with a response, and this function will not wait for any.
	///
	/// If you want to broadcast this instruction, it may be more convenient to use [`Self::broadcast_factory_reset()`] instead.
	///
	/// Starting with version 42 of the firmware for the MX-series and X-series,
	/// motors ignore a broadcast reset command with `FactoryResetKind::ResetAll`.
	/// Motors with older firmware may still execute the command,
	/// which would cause multiple motors on the bus to have the same ID.
	/// At that point, communication with those motors is not possible anymore.
	/// The only way to restore communication is to physically disconnect all but one motor at a time and re-assign unique IDs.
	/// Or use the ID Inspection Tool in the Dynamixel Wizard 2.0
	pub fn factory_reset(&mut self, motor_id: u8, kind: FactoryResetKind) -> Result<Response<()>, TransferError<T::Error>> {
		self.write_instruction(motor_id, instruction_id::FACTORY_RESET, 1, |buffer| buffer[0] = kind as u8)?;
		Ok(super::read_response_if_not_broadcast(self, motor_id)?)
	}

	/// Reset the settings of all connected motors to the factory defaults.
	///
	/// This will reset all registers to the factory default, including the EEPROM registers.
	/// The only exceptions are the ID and baud rate settings, which may be kept depending on the `kind` argument.
	///
	/// Starting with version 42 of the firmware for the MX-series and X-series,
	/// motors ignore a broadcast reset command with `FactoryResetKind::ResetAll`.
	/// Motors with older firmware may still execute the command,
	/// which would cause multiple motors on the bus to have the same ID.
	/// At that point, communication with those motors is not possible anymore.
	/// The only way to restore communication is to physically disconnect all but one motor at a time and re-assign unique IDs.
	pub fn broadcast_factory_reset(&mut self, kind: FactoryResetKind) -> Result<(), WriteError<T::Error>> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::FACTORY_RESET, 1, |buffer| {
			buffer[0] = kind as u8
		})?;
		Ok(())
	}
}
