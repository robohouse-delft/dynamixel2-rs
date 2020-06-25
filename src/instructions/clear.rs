use super::{instruction_id, packet_id, Instruction};

#[derive(Debug, Clone)]
pub struct ClearMultiTurnCounter {
	pub motor_id: u8,
}

impl ClearMultiTurnCounter {
	pub fn unicast(motor_id: u8) -> Self {
		Self { motor_id }
	}

	pub fn broadcast() -> Self {
		Self {
			motor_id: packet_id::BROADCAST,
		}
	}
}

const CLEAR_MULTI_TURN_COUNTER_PARAMS: [u8; 5] = [0x01, 0x44, 0x58, 0x4C, 0x22];

impl Instruction for ClearMultiTurnCounter {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::CLEAR
	}

	fn request_parameters_len(&self) -> u16 {
		CLEAR_MULTI_TURN_COUNTER_PARAMS.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		buffer.copy_from_slice(&CLEAR_MULTI_TURN_COUNTER_PARAMS);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;

		Ok(())
	}
}
