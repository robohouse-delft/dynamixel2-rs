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
//! There is also an [`AsyncClient`] for use with an asynchronous serial port,
//! and a [`Device`] and [`AsyncDevice`] to implement the device side of the protocol.
//!
//! The library currently implements all instructions except for the Control Table Backup and Fast Sync Write instructions.
//!
//! # Optional features
//!
//! You can enable the `log` feature to have the library use `log::trace!()` to log all sent instructions and received replies.
//!
//! # Example
//!
//! For example, to ping a motor using the synchronous client:
//! ```no_run
//! # #[cfg(feature = "serial2")]
//! # fn do_main() -> Result<(), Box<dyn std::error::Error>> {
//! type Client = dynamixel2::Client<serial2::SerialPort>;
//! let mut client = Client::open("/dev/ttyUSB0", 115200)?;
//! let response = client.ping(1)?;
//! println!("{response:#?}");
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![allow(clippy::duplicate_mod)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod log;

pub mod checksum;

pub mod bus;

pub mod client;
pub use client::{AsyncClient, Client};

mod error;
pub use error::*;

mod response;
pub use response::*;

pub mod device;
pub use device::{AsyncDevice, Device};

#[cfg(feature = "serial2")]
/// Public re-export of the serial2 crate.
pub use serial2;

#[cfg(feature = "serial2-tokio")]
/// Public re-export of the serial2 crate.
pub use serial2_tokio;

mod serial_port;
pub use serial_port::*;
