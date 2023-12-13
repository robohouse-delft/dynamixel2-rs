#[rustfmt::skip]
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

pub mod packet_id {
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

pub use factory_reset::FactoryResetKind;
pub use ping::PingResponse;

use crate::Response;

/// Data from or for a specific motor.
///
/// Used by synchronous read and write commands.
pub struct SyncData<T> {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The data read from or to be written to the motor.
	pub data: T,
}

impl<T> AsRef<SyncData<T>> for SyncData<T> {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// Bulk data for a specific motor.
///
/// This struct is very comparable to [`SyncData`],
/// but it supports reads and writes
/// of different sizes and to different addresses for each motor.
///
/// Used by bulk read and write commands.
pub struct BulkData<T> {
	/// The ID of the motor.
	pub motor_id: u8,

	/// The address for the read or write.
	pub address: u16,

	/// The data read from or to be written to the motor.
	pub data: T,
}

impl<T> AsRef<BulkData<T>> for BulkData<T> {
	fn as_ref(&self) -> &Self {
		self
	}
}

/// The data response giving by a motor in response to a bulk read command.
#[derive(Debug)]
pub struct BulkResponse<T> {
	/// The response message.
	pub response: Response<T>,
	
	/// The address of the bulk read/write.
	pub address: u16,
}
