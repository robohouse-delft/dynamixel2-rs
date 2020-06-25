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

pub trait Instruction {
	/// The response type.
	type Response;

	/// The packet ID.
	fn request_packet_id(&self) -> u8;

	/// The instruction ID for the request.
	fn request_instruction_id(&self) -> u8;

	/// The amount of parameter bytes before bitstuffing.
	fn request_parameters_len(&self) -> u16;

	/// Encode the request parameters to the target buffer.
	///
	/// The buffer is guaranteed to be atleast as large
	/// as returned by [`Instruction::request_parameters_len`].
	fn encode_request_parameters(&self, buffer: &mut [u8]);

	/// Decode the response from the parameters.
	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage>;
}

mod action;
mod clear;
mod factory_reset;
mod ping;
mod raw;
mod read;
mod reboot;
mod reg_write;
mod write;

pub use action::Action;
pub use clear::{ClearMultiTurnCounter};
pub use factory_reset::{FactoryReset, FactoryResetKind};
pub use ping::{Ping, PingResponse};
pub use raw::{Raw, RawResponse};
pub use read::{Read, ReadU16, ReadU32, ReadU8};
pub use reboot::Reboot;
pub use reg_write::{RegWrite, RegWriteU16, RegWriteU32, RegWriteU8};
pub use write::{Write, WriteU16, WriteU32, WriteU8};
