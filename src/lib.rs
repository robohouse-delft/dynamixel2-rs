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
//! The main interface is the [`Client`] struct, which can be used to interact with devices on the serial communication bus.
//! The [`Client`] struct exposes functions for all supported instructions such as [`Client::ping`], [`Client::read`], [`Client::write`] and much more.
//! Additionally, you can also transmit raw commands using [`Client::write_instruction`] and [`Client::read_status_response`], or [`Client::transfer_single`].
//!
//! The library currently implements all instructions except for the Control Table Backup, Fast Sync Read and Fast Sync Write instructions.
//!
//! # Optional features
//!
//! You can enable the `log` feature to have the library use `log::trace!()` to log all sent instructions and received replies.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "serial2")]
/// Public re-export of the serial2 crate.
pub use serial2;

#[macro_use]
mod log;

pub mod checksum;
pub mod instructions;

mod client;
pub use client::*;

mod device;
pub use device::*;

mod serial_port;
pub use serial_port::SerialPort;

mod error;
pub use error::*;

mod packet;
pub use packet::Packet;

mod response;
pub use response::*;

pub mod bus;

mod bytestuff;
mod endian;
