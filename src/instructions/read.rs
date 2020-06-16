use super::{id, Instruction};
use crate::endian::write_u16_le;

#[derive(Debug)]
pub struct Read<'a> {
	pub motor_id: u8,
	pub address: u16,
	pub buffer: &'a mut [u8],
}

impl<'a> Read<'a> {
	pub fn new(motor_id: u8, address: u16, buffer: &'a mut [u8]) -> Self {
		Self { motor_id, address, buffer }
	}
}

impl Instruction for Read<'_> {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		id::READ
	}

	fn request_parameters_len(&self) -> u16 {
		4
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..], self.address);
		write_u16_le(&mut buffer[2..], self.buffer.len() as u16);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::ReadError> {
		if packet_id != self.motor_id {
			return Err(crate::ReadError::InvalidPacketId);
		}

		if parameters.len() != self.buffer.len() {
			return Err(crate::ReadError::InvalidParameterCount);
		}

		self.buffer.copy_from_slice(parameters);
		Ok(())
	}
}
