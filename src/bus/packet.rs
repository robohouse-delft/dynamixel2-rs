/// A packet (either an instruction packet or a status packet) in the buffer of the client/device
#[derive(Debug, Copy, Clone)]
pub struct Packet<'a> {
	/// Message data (with byte-stuffing already undone).
	///
	/// Does not include the CRC checksum.
	pub(crate) data: &'a [u8],
}

impl<'a> Packet<'a> {
	/// The packet ID.
	pub fn packet_id(self) -> u8 {
		self.data[4]
	}

	/// The instruction ID.
	pub fn instruction_id(self) -> u8 {
		self.data[7]
	}

	/// Get the packet as a [`StatusPacket`], if it is one.
	pub fn as_status(self) -> Option<StatusPacket<'a>> {
		if self.instruction_id() == crate::instructions::instruction_id::STATUS {
			Some(StatusPacket { packet: self })
		} else {
			None
		}
	}

	/// Get the packet as a [`InstructionPacket`], if it is one.
	pub fn as_instruction(self) -> InstructionPacket<'a> {
		InstructionPacket { packet: self }
	}
}

/// A [`StatusPacket`] contains an error byte and response parameters to an [`InstructionPacket`]
///
/// Sent by a device to the client.

#[derive(Debug, Copy, Clone)]
pub struct StatusPacket<'a> {
	packet: Packet<'a>,
}

impl<'a> StatusPacket<'a> {
	/// The error field of the response.
	pub fn error(self) -> u8 {
		self.packet.data[8]
	}

	/// The error number of the status packet.
	///
	/// This is the lower 7 bits of the error field.
	pub fn error_number(self) -> u8 {
		self.error() & !0x80
	}

	/// The alert bit from the error field of the response.
	///
	/// This is the 8th bit of the error field.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub fn alert(self) -> bool {
		self.error() & 0x80 != 0
	}

	/// The packet ID.
	pub fn packet_id(self) -> u8 {
		self.packet.packet_id()
	}

	/// The instruction ID.
	pub fn instruction_id(self) -> u8 {
		self.packet.instruction_id()
	}

	/// The parameters of the packet.
	pub fn parameters(self) -> &'a [u8] {
		&self.packet.data[super::HEADER_SIZE + 2..]
	}

	/// Calculate the size of a (unstuffed) status message with the given number of parameters.
	pub(crate) const fn message_len(parameters: usize) -> usize {
		super::HEADER_SIZE + 2 + parameters + 2
	}
}

/// [`InstructionPacket`] is a packet that contains an instruction and its parameters.
///
/// Sent by a client to a device.
#[derive(Debug, Copy, Clone)]
pub struct InstructionPacket<'a> {
	packet: Packet<'a>,
}

impl<'a> InstructionPacket<'a> {
	/// The packet ID.
	pub fn packet_id(self) -> u8 {
		self.packet.packet_id()
	}

	/// The instruction ID.
	pub fn instruction_id(self) -> u8 {
		self.packet.instruction_id()
	}

	/// The parameters of the packet.
	pub fn parameters(self) -> &'a [u8] {
		&self.packet.data[super::HEADER_SIZE + 1..]
	}

	/// Calculate the size of a (unstuffed) instruction message with the given number of parameters.
	pub(crate) const fn message_len(parameters: usize) -> usize {
		super::HEADER_SIZE + 1 + parameters + 2
	}
}

#[cfg(test)]
mod test {
	use assert2::{assert, let_assert};

	use super::*;
	use crate::instructions::instruction_id;

	#[test]
	fn status_packet() {
		let packet = Packet {
			data: &[0xFF, 0xFF, 0xFD, 0x00, 0x23, 0x04, 0x00, instruction_id::STATUS, 0x89],
		};
		assert!(packet.packet_id() == 0x23);
		assert!(packet.instruction_id() == instruction_id::STATUS);
		let_assert!(Some(status) = packet.as_status());
		assert!(status.packet_id() == 0x23);
		assert!(status.instruction_id() == instruction_id::STATUS);
		assert!(status.error() == 0x89);
		assert!(status.parameters() == &[]);
	}

	#[test]
	fn instruction_packet() {
		let packet = Packet {
			data: &[0xFF, 0xFF, 0xFD, 0x00, 0x23, 0x03, 0x00, instruction_id::WRITE],
		};
		assert!(packet.packet_id() == 0x23);
		assert!(packet.instruction_id() == instruction_id::WRITE);
		let instruction = packet.as_instruction();
		assert!(instruction.packet_id() == 0x23);
		assert!(instruction.instruction_id() == instruction_id::WRITE);
		assert!(instruction.parameters() == &[]);
	}
}
