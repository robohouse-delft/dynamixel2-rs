//! Types and functions for specific instructions.
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
