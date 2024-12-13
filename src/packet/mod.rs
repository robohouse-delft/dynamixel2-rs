mod status;
pub use status::{StatusPacket, Response};
mod instruction;
pub use instruction::{Instruction, Instructions, InstructionPacket};

pub(crate) const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
pub(crate) const INSTRUCTION_HEADER_SIZE: usize = 8;
pub(crate) const STATUS_HEADER_SIZE: usize = 9;

/// A trait for both [`InstructionPacket`]s and [`StatusPacket`]s that can be sent and received.
pub trait Packet<'a> {
	/// The size of the packet header in bytes.
	const HEADER_SIZE: usize;

	/// The number of bytes included in both the packet length and [`Self::HEADER_SIZE`].
	const HEADER_OVERLAP: usize;

	/// Create a new packet from raw data.
	fn new(data: &'a [u8]) -> Self;

	/// Get the raw bytes of the message.
	///
	/// This includes the message header and the parameters.
	/// It does not include the CRC or byte-stuffing.
	fn as_bytes(&'a self) -> &'a [u8];

	/// The packet ID of the response.
	fn packet_id(&'a self) -> u8 {
		self.as_bytes()[4]
	}

	/// The instruction ID of the response.
	fn instruction_id(&'a self) -> u8 {
		self.as_bytes()[7]
	}
	/// The parameters of the response.
	fn parameters(&'a self) -> &'a [u8] {
		&self.as_bytes()[Self::HEADER_SIZE..]
	}
}

impl<'a> Packet<'a> for InstructionPacket<'a> {
	const HEADER_SIZE: usize = INSTRUCTION_HEADER_SIZE;
	const HEADER_OVERLAP: usize = 1;

	fn new(data: &'a [u8]) -> Self {
		Self { data }
	}

	fn as_bytes(&'a self) -> &'a [u8] {
		self.data
	}
}

impl<'a> Packet<'a> for StatusPacket<'a> {
	const HEADER_SIZE: usize = STATUS_HEADER_SIZE;

	const HEADER_OVERLAP: usize = 2;

	fn new(data: &'a [u8]) -> Self {
		Self { data }
	}

	fn as_bytes(&'a self) -> &'a [u8] {
		self.data
	}
}
