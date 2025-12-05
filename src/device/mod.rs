//! [`Device`] and [`AsyncDevice`] are used to receive communication from a client and respond

use crate::bus::endian::read_u16_le;
use crate::bus::{instruction_id, InstructionPacket};
use crate::InvalidParameterCount;

#[cfg(feature = "alloc")]
use alloc::borrow::ToOwned;

#[path = "."]
pub(crate) mod asynch {
	use crate::bus::asynch::Bus;
	use crate::AsyncSerialPort as SerialPort;
	use bisync::asynchronous::*;
	#[cfg(feature = "serial2")]
	use serial2_tokio::SerialPort as Serial2Port;

	pub(super) mod device;
}
#[path = "."]
pub(super) mod sync {
	use crate::bus::sync::Bus;
	use crate::SerialPort;
	use bisync::synchronous::*;
	#[cfg(feature = "serial2")]
	use serial2::SerialPort as Serial2Port;

	pub(super) mod device;
}

pub use asynch::device::Device as AsyncDevice;
pub use sync::device::Device;

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
