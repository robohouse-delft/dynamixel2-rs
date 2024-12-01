//! Types and functions for specific instructions.

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

mod action;
mod bulk_read;
mod bulk_write;
mod clear;
mod factory_reset;
mod ping;
mod read;
mod reboot;
mod reg_write;
mod sync_read;
mod sync_write;
mod write;

use crate::SerialPort;
pub use factory_reset::FactoryResetKind;
pub use ping::Ping;

/// Data from or for a specific motor.
///
/// Used by synchronous write commands.
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
/// Used by bulk write commands.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkWriteData<T> {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The address for the read or write.
	pub address: u16,

	/// The data read from or to be written to the motor.
	pub data: T,
}

impl<T> AsRef<BulkWriteData<T>> for BulkWriteData<T> {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// Parameters for a bulk read instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BulkReadData {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The address for the read or write.
	pub address: u16,

	/// The length of the data to be read.
	pub count: u16,
}

impl AsRef<BulkReadData> for BulkReadData {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// Read an empty response from the bus if the motor ID is not the broadcast ID.
///
/// If the motor ID is the broadcast ID, return a fake response from the broadcast ID.
fn read_response_if_not_broadcast<ReadBuffer, WriteBuffer, T>(
	client: &mut crate::Client<ReadBuffer, WriteBuffer, T>,
	motor_id: u8,
) -> Result<crate::Response<()>, crate::error::ReadError<T::Error>>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	if motor_id == packet_id::BROADCAST {
		Ok(crate::Response {
			motor_id: packet_id::BROADCAST,
			alert: false,
			data: (),
		})
	} else {
		Ok(client.read_status_response(0)?.try_into()?)
	}
}
