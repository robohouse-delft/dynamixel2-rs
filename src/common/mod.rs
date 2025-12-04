use crate::{bus::StatusPacket, Response};

/// Raw instructions IDs.
#[rustfmt::skip]
#[allow(missing_docs)]
pub mod instruction_id {
	pub const PING          : u8 = 0x01;
	pub const READ          : u8 = 0x02;
	pub const WRITE         : u8 = 0x03;
	pub const REG_WRITE     : u8 = 0x04;
	pub const ACTION        : u8 = 0x05;
	pub const FACTORY_RESET : u8 = 0x06;
	pub const REBOOT        : u8 = 0x08;
	pub const CLEAR         : u8 = 0x10;
	pub const SYNC_READ     : u8 = 0x82;
	pub const SYNC_WRITE    : u8 = 0x83;
	pub const BULK_READ     : u8 = 0x92;
	pub const BULK_WRITE    : u8 = 0x93;
	pub const STATUS        : u8 = 0x55;
}

/// Special packet IDs.
pub mod packet_id {
	/// The broadcast address.
	pub const BROADCAST: u8 = 0xFE;
}

/// Sync data for a specific motor.
///
/// Used by [`crate::Client::sync_write`] and [`crate::Client::sync_write_bytes`]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SyncWriteData<T> {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The data to be written to the motor.
	pub data: T,
}

impl<T> AsRef<SyncWriteData<T>> for SyncWriteData<T> {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// Bulk data for a specific motor.
///
/// This struct is similar to [`SyncWriteData`],
/// but it supports reads and writes of different sizes and to different addresses for each motor.
///
/// Used with [`crate::Client::bulk_write`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkWriteData<T> {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The address to be written to
	pub address: u16,

	/// The data to be written
	pub data: T,
}

impl<T> AsRef<BulkWriteData<T>> for BulkWriteData<T> {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// Parameters for a bulk read instruction.
///
/// Use with [`crate::Client::bulk_read_bytes`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct BulkReadData {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The address of the data to be read
	pub address: u16,

	/// The length of the data to be read.
	pub count: u16,
}

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

/// A response from a motor to a ping instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Ping {
	/// The model of the motor.
	///
	/// Refer to the online manual to find the codes for each model.
	pub model: u16,

	/// The firmware version of the motor.
	pub firmware: u8,
}

impl<'a> TryFrom<StatusPacket<'a>> for Response<Ping> {
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a>) -> Result<Self, Self::Error> {
		let parameters = status_packet.parameters();
		crate::InvalidParameterCount::check(parameters.len(), 3)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: Ping {
				model: crate::bus::endian::read_u16_le(&parameters[0..]),
				firmware: crate::bus::endian::read_u8_le(&parameters[2..]),
			},
		})
	}
}

use crate::bus::endian::read_u16_le;
use crate::bus::InstructionPacket;
use crate::InvalidParameterCount;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

/// The options for the [Factory Reset](https://emanual.robotis.com/docs/en/dxl/protocol2/#factory-reset-0x06) instruction.
#[derive(Debug)]
pub enum FactoryReset {
	/// Reset all values to their factory defaults.
	All,
	/// Resets all values, except the ID.
	ExceptId,
	/// Resets all values, except the ID and baud rate.
	ExceptIdBaudRate,
	/// Reserved for future use.
	Unknown(u8),
}

/// The options for the [Clear](https://emanual.robotis.com/docs/en/dxl/protocol2/#clear-0x10) instruction.
#[derive(Debug)]
pub enum Clear {
	/// Reset the Present Position value to an absolute value within one rotation (0-4095).
	MultiTurns,
	/// Clear errors that occurred in DYNAMIXEL.
	/// If an error cannot be cleared or the conditions for clearance are not met, the error remains uncleared, and Result Fail (0x01) is displayed in the Error field of the Status Packet.
	/// Support only DYNAMIXEL Y series.
	Errors,
	/// Reserved for future use.
	Reserved(u8),
}

/// [`InstructionPacket`] can be converted into an [`Instruction`] with borrowed or owned data.
/// It contains the ID and parameters.
/// The owned data variant requires the `alloc` feature.
///
#[derive(Debug)]
pub struct Instruction<T> {
	/// The ID of the packet
	pub id: u8,
	/// The instruction as parsed into the [`Instructions`] enum
	pub instruction: Instructions<T>,
}

/// Instructions as defined in the [Dynamixel Protocol 2.0](https://emanual.robotis.com/docs/en/dxl/protocol2/#instruction-details).
///
/// The parameters are stored as a `&[u8]` slice or a `Vec<u8>`.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Instructions<T> {
	Ping,
	Read { address: u16, length: u16 },
	Write { address: u16, parameters: T },
	RegWrite { address: u16, parameters: T },
	Action,
	FactoryReset(FactoryReset),
	Reboot,
	Clear(Clear),
	SyncRead { address: u16, length: u16, ids: T },
	SyncWrite { address: u16, length: u16, parameters: T },
	BulkRead { parameters: T },
	BulkWrite { parameters: T },
	StatusPacket { error: u8, parameters: T },
	Unknown { instruction: u8, parameters: T },
}

impl<'a> TryFrom<InstructionPacket<'a>> for Instruction<&'a [u8]> {
	type Error = InvalidParameterCount;

	fn try_from(packet: InstructionPacket<'a>) -> Result<Self, Self::Error> {
		let id = packet.packet_id();
		let parameters: &'a [u8] = packet.parameters();
		let instruction = match packet.instruction_id() {
			instruction_id::PING => Instructions::Ping,
			instruction_id::READ => {
				InvalidParameterCount::check(parameters.len(), 4)?;
				Instructions::Read {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
				}
			},
			instruction_id::WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 2)?;
				Instructions::Write {
					address: read_u16_le(&parameters[..2]),
					parameters: &parameters[2..],
				}
			},
			instruction_id::REG_WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 2)?;
				Instructions::RegWrite {
					address: read_u16_le(&parameters[..2]),
					parameters: &parameters[2..],
				}
			},
			instruction_id::ACTION => Instructions::Action,
			instruction_id::FACTORY_RESET => {
				InvalidParameterCount::check(parameters.len(), 1)?;
				let reset_type = match parameters[0] {
					0xFF => FactoryReset::All,
					0x01 => FactoryReset::ExceptId,
					0x02 => FactoryReset::ExceptIdBaudRate,
					p => FactoryReset::Unknown(p),
				};
				Instructions::FactoryReset(reset_type)
			},
			instruction_id::REBOOT => Instructions::Reboot,
			instruction_id::CLEAR => {
				InvalidParameterCount::check_min(parameters.len(), 1)?;
				match parameters[0] {
					0x01 => Instructions::Clear(Clear::MultiTurns),
					0x02 => Instructions::Clear(Clear::Errors),
					p => Instructions::Clear(Clear::Reserved(p)),
				}
			},
			// todo: instruction_id::ControlTableBackup
			instruction_id::SYNC_READ => {
				InvalidParameterCount::check_min(parameters.len(), 4)?;
				Instructions::SyncRead {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
					ids: &parameters[4..],
				}
			},
			instruction_id::SYNC_WRITE => {
				InvalidParameterCount::check_min(parameters.len(), 4)?;
				Instructions::SyncWrite {
					address: read_u16_le(&parameters[..2]),
					length: read_u16_le(&parameters[2..4]),
					parameters: &parameters[4..],
				}
			},
			instruction_id::BULK_READ => Instructions::BulkRead { parameters },
			instruction_id::BULK_WRITE => Instructions::BulkWrite { parameters },

			instruction_id::STATUS => {
				InvalidParameterCount::check_min(parameters.len(), 1)?;

				let error = parameters[0];
				Instructions::StatusPacket {
					error,
					parameters: &parameters[1..],
				}
			},

			instruction => Instructions::Unknown { instruction, parameters },
		};

		Ok(Instruction { id, instruction })
	}
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a> TryFrom<InstructionPacket<'a>> for Instruction<alloc::vec::Vec<u8>> {
	type Error = InvalidParameterCount;

	fn try_from(packet: InstructionPacket<'a>) -> Result<Self, Self::Error> {
		let packet: Instruction<&[u8]> = packet.try_into()?;
		let Instruction { id, instruction } = packet;
		let instruction = match instruction {
			Instructions::Ping => Instructions::Ping,
			Instructions::Read { address, length } => Instructions::Read { address, length },
			Instructions::Write { address, parameters } => Instructions::Write {
				address,
				parameters: parameters.to_owned(),
			},
			Instructions::RegWrite { address, parameters } => Instructions::RegWrite {
				address,
				parameters: parameters.to_owned(),
			},
			Instructions::Action => Instructions::Action,
			Instructions::FactoryReset(f) => Instructions::FactoryReset(f),
			Instructions::Reboot => Instructions::Reboot,
			Instructions::Clear(c) => Instructions::Clear(c),
			Instructions::SyncRead { address, length, ids } => Instructions::SyncRead {
				address,
				length,
				ids: ids.to_owned(),
			},
			Instructions::SyncWrite {
				address,
				length,
				parameters,
			} => Instructions::SyncWrite {
				address,
				length,
				parameters: parameters.to_owned(),
			},
			Instructions::BulkRead { parameters } => Instructions::BulkRead {
				parameters: parameters.to_owned(),
			},
			Instructions::BulkWrite { parameters } => Instructions::BulkRead {
				parameters: parameters.to_owned(),
			},
			Instructions::StatusPacket { error, parameters } => Instructions::StatusPacket {
				error,
				parameters: parameters.to_owned(),
			},
			Instructions::Unknown { instruction, parameters } => Instructions::Unknown {
				instruction,
				parameters: parameters.to_owned(),
			},
		};
		Ok(Instruction { id, instruction })
	}
}
