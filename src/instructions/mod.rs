pub mod id {
	pub const PING:  u8 = 1;
	pub const READ:  u8 = 2;
	pub const WRITE: u8 = 3;
	pub const REG_WRITE: u8 = 4;
	pub const ACTION: u8 = 5;
	pub const FACTORY_RESET: u8 = 6;
	pub const REBOOT: u8 = 7;
	pub const CLEAR: u8 = 8;
	pub const SYNC_READ: u8 = 9;
	pub const SYNC_WRITE: u8 = 10;
	pub const BULK_READ: u8 = 11;
	pub const BULK_WRITE: u8 = 12;
	pub const STATUS: u8 = 0x55;
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
	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::ReadError>;
}

mod ping;
mod read;
mod write;

pub use ping::{Ping, PingResponse};
pub use read::Read;
pub use write::Write;
