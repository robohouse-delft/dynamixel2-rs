use crate::packet::STATUS_HEADER_SIZE;

/// A status response that is currently in the read buffer of a client.
#[derive(Debug)]
pub struct StatusPacket<'a> {
	/// Message data (with byte-stuffing already undone).
	pub(crate) data: &'a [u8],
}

impl<'a> StatusPacket<'a> {
	/// The error field of the response.
	pub fn error(&self) -> u8 {
		self.data[8]
	}

	/// The error number of the status packet.
	///
	/// This is the lower 7 bits of the error field.
	pub fn error_number(&self) -> u8 {
		self.error() & !0x80
	}

	/// The alert bit from the error field of the response.
	///
	/// This is the 8th bit of the error field.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub fn alert(&self) -> bool {
		self.error() & 0x80 != 0
	}

	/// The packet ID.
	pub fn packet_id(&'a self) -> u8 {
		self.data[4]
	}

	/// The instruction ID.
	pub fn instruction_id(&'a self) -> u8 {
		self.data[7]
	}

	/// The parameters of the packet.
	pub fn parameters(&'a self) -> &'a [u8] {
		&self.data[STATUS_HEADER_SIZE..]
	}
}
