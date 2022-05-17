use super::{instruction_id, packet_id};
use crate::Bus;

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

#[cfg(feature = "sync")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Reset the settings of a motor to the factory defaults.
	///
	/// This will reset all registers to the factory default, including the EEPROM registers.
	/// The only exceptions are the ID and baud rate settings, which may be kept depending on the `kind` argument.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_factory_reset`] is generally easier to use.
	///
	/// Starting with version 42 of the firmware for the MX-series and X-series,
	/// motors ignore a broadcast reset command with `FactoryResetKind::ResetAll`.
	/// Motors with older firmware may still execute the command,
	/// which would cause multiple motors on the bus to have the same ID.
	/// At that point, communication with those motors is not possible anymore.
	/// The only way to restore communication is to physically disconnect all but one motor at a time and re-assign unique IDs.
	pub fn factory_reset(&mut self, motor_id: u8, kind: FactoryResetKind) -> Result<(), crate::TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_factory_reset(kind)?;
		} else {
			let response = self.transfer_single(motor_id, instruction_id::FACTORY_RESET, 1, |buffer| buffer[0] = kind as u8)?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
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
	pub fn broadcast_factory_reset(&mut self, kind: FactoryResetKind) -> Result<(), crate::WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::FACTORY_RESET, 1, |buffer| {
			buffer[0] = kind as u8
		})?;
		Ok(())
	}
}

#[cfg(any(feature = "async_smol", feature = "async_tokio"))]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Reset the settings of a motor to the factory defaults.
	///
	/// This will reset all registers to the factory default, including the EEPROM registers.
	/// The only exceptions are the ID and baud rate settings, which may be kept depending on the `kind` argument.
	///
	/// The `motor_id` parameter may be set to [`packet_id::BROADCAST`],
	/// although the [`Self::broadcast_factory_reset`] is generally easier to use.
	///
	/// Starting with version 42 of the firmware for the MX-series and X-series,
	/// motors ignore a broadcast reset command with `FactoryResetKind::ResetAll`.
	/// Motors with older firmware may still execute the command,
	/// which would cause multiple motors on the bus to have the same ID.
	/// At that point, communication with those motors is not possible anymore.
	/// The only way to restore communication is to physically disconnect all but one motor at a time and re-assign unique IDs.
	pub async fn factory_reset(&mut self, motor_id: u8, kind: FactoryResetKind) -> Result<(), crate::TransferError> {
		if motor_id == packet_id::BROADCAST {
			self.broadcast_factory_reset(kind).await?;
		} else {
			let response = self
				.transfer_single(motor_id, instruction_id::FACTORY_RESET, 1, |buffer| buffer[0] = kind as u8)
				.await?;
			crate::InvalidParameterCount::check(response.parameters().len(), 0).map_err(crate::ReadError::from)?;
		}
		Ok(())
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
	pub async fn broadcast_factory_reset(&mut self, kind: FactoryResetKind) -> Result<(), crate::WriteError> {
		self.write_instruction(packet_id::BROADCAST, instruction_id::FACTORY_RESET, 1, |buffer| {
			buffer[0] = kind as u8
		})
		.await?;
		Ok(())
	}
}
