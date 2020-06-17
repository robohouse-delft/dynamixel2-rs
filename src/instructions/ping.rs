use super::{instruction_id, packet_id, Instruction};
use crate::endian::read_u16_le;

#[derive(Debug, Clone)]
pub struct Ping {
	pub motor_id: u8,
}

#[derive(Debug, Clone)]
pub struct PingResponse {
	pub motor_id: u8,
	pub model: u16,
	pub firmware: u8,
}

impl Ping {
	pub fn unicast(motor_id: u8) -> Self {
		Self { motor_id }
	}

	pub fn broadcast() -> Self {
		Self {
			motor_id: packet_id::BROADCAST,
		}
	}
}

impl Instruction for Ping {
	type Response = PingResponse;

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::PING
	}

	fn request_parameters_len(&self) -> u16 {
		0
	}

	fn encode_request_parameters(&self, _buffer: &mut [u8]) {
		// Empty parameters.
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidParameterCount::check(parameters.len(), 3)?;

		Ok(Self::Response {
			motor_id: packet_id,
			model: read_u16_le(&parameters[0..]),
			firmware: parameters[2],
		})
	}
}
