use super::{id, Instruction};
use crate::endian::write_u16_le;

#[derive(Debug, Clone)]
pub struct Write<'a> {
	pub motor_id: u8,
	pub address: u16,
	pub data: &'a[u8],
}

impl<'a> Write<'a> {
	pub fn new(motor_id: u8, address: u16, data: &'a [u8]) -> Self {
		Self { motor_id, address, data }
	}
}

impl Instruction for Write<'_> {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		id::WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		2 + self.data.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..], self.address);
		buffer[2..].copy_from_slice(&self.data);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}
