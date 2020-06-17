use super::{instruction_id, Instruction, packet_id};

#[derive(Debug, Clone)]
pub struct Reboot {
	pub motor_id: u8,
}

impl Reboot {
	pub fn unicast(motor_id: u8) -> Self {
		Self { motor_id }
	}

	pub fn broadcast() -> Self {
		Self { motor_id: packet_id::BROADCAST }
	}
}

impl Instruction for Reboot {
	type Response = ();

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
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}
