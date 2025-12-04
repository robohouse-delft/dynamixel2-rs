//! Types and functions for specific instructions.
use crate::error::ReadError;

use super::{bisync, client::Client, only_sync, SerialPort};

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

#[super::only_sync]
pub use ping::Scan;
#[super::only_sync]
pub use sync_read::SyncRead;

/// Read an empty response from the bus if the motor ID is not the broadcast ID.
///
/// If the motor ID is the broadcast ID, return a fake response from the broadcast ID.
#[bisync]
async fn read_response_if_not_broadcast<SerialPort, Buffer>(
	client: &mut super::Client<SerialPort, Buffer>,
	motor_id: u8,
) -> Result<crate::Response<()>, ReadError<SerialPort::Error>>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	if motor_id == crate::packet_id::BROADCAST {
		Ok(crate::Response {
			motor_id: crate::packet_id::BROADCAST,
			alert: false,
			data: (),
		})
	} else {
		Ok(client.read_status_response(0).await?.try_into()?)
	}
}
