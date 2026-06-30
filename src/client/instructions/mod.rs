//! Types and functions for specific instructions.
use super::{bisync, client::Client, only_sync, SerialPort};

mod action;
pub(crate) mod bulk_read;
mod bulk_write;
mod clear;
mod factory_reset;
pub(crate) mod ping;
mod read;
mod reboot;
mod reg_write;
pub(crate) mod sync_read;
mod sync_write;
mod write;
