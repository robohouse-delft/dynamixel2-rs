//! [`Client`] and [`AsyncClient`] are used to communicate with devices

use crate::{bus::StatusPacket, Response};

#[path = "."]
pub(crate) mod asynch {
	use crate::bus::asynch::Bus;
	use crate::AsyncSerialPort as SerialPort;
	use bisync::asynchronous::*;

	mod client;
	pub use client::Client;
	pub(crate) mod instructions;
}

#[path = "."]
pub(crate) mod sync {
	use crate::bus::sync::Bus;
	use crate::SerialPort;
	use bisync::synchronous::*;

	mod client;
	pub use client::Client;
	pub(crate) mod instructions;
}

pub use asynch::instructions::bulk_read::BulkReadBytes as AsyncBulkReadBytes;
pub use asynch::instructions::ping::Scan as AsyncScan;
pub use asynch::instructions::{
	fast_bulk_read::FastBulkRead as AsyncFastBulkRead, fast_sync_read::FastSyncRead as AsyncFastSyncRead,
	fast_sync_read::FastSyncReadBytes as AsyncFastSyncReadBytes,
};
pub use asynch::instructions::{sync_read::SyncRead as AsyncSyncRead, sync_read::SyncReadBytes as AsyncSyncReadBytes};
pub use asynch::Client as AsyncClient;

pub use sync::instructions::{
	bulk_read::BulkReadBytes, fast_bulk_read::FastBulkRead, fast_sync_read::FastSyncRead, fast_sync_read::FastSyncReadBytes, ping::Scan,
	sync_read::SyncRead, sync_read::SyncReadBytes,
};
pub use sync::Client;

/// Sync data for a specific motor.
///
/// Used by [`Client::sync_write`] and [`Client::sync_write_bytes`]
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
/// Used with [`Client::bulk_write`].
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
/// Use with [`Client::bulk_read_bytes`].
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
