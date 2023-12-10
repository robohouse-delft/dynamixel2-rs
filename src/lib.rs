//! An implementation of the [Dynamixel Protocol 2.0].
//!
//! [Dynamixel Protocol 2.0]: https://emanual.robotis.com/docs/en/dxl/protocol2/
//!
//! This library aims to provide a easy to use but low level implementation of the Dynamixel Protocol 2.0.
//! That means it allows you to execute arbitrary commands with arbitrary parameters.
//!
//! The library does not aim to provide an easy interface to the higher level functions of a servo motor,
//! such as moving it to a specific angle or at a specific speed.
//! Instead, you will have to write the appropriate values to the correct registers yourself.
//!
//! The main interface is the [`Bus`] struct, which represents the serial communication bus.
//! The [`Bus`] struct exposes functions for all supported instructions such as [`Bus::ping`], [`Bus::read`], [`Bus::write`] and much more.
//! Additionally, you can also transmit raw commands using [`Bus::write_instruction`] and [`Bus::read_status_response`], or [`Bus::transfer_single`].
//!
//! The library currently implements all instructions except for the Control Table Backup, Fast Sync Read and Fast Sync Write instructions.
//!
//! # Optional features
//!
//! You can enable the `log` feature to have the library use `log::trace!()` to log all sent instructions and received replies.

#[macro_use]
mod log;

pub mod checksum;
pub mod instructions;

mod bus;
mod bytestuff;
mod endian;
mod error;

pub use error::InvalidChecksum;
pub use error::InvalidHeaderPrefix;
pub use error::InvalidInstruction;
pub use error::InvalidMessage;
pub use error::InvalidPacketId;
pub use error::InvalidParameterCount;
pub use error::MotorError;
pub use error::ReadError;
pub use error::TransferError;
pub use error::WriteError;

pub use bus::Bus;
pub use bus::Response;
pub use instructions::BulkResponse;
