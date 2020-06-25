use super::Instruction;

/// A raw instruction.
pub struct Raw<'a> {
	/// The packet ID.
	pub packet_id: u8,

	/// The instruction ID.
	pub instruction_id: u8,

	/// The instruction parameters.
	pub instruction_params: &'a [u8],

	/// Buffer for the response parameters.
	pub response_params: &'a mut [u8],
}

pub struct RawResponse {
	/// The packet ID from the response.
	pub packet_id: u8,

	/// The number of parameters in the response.
	pub parameters_len: usize,
}

impl Instruction for Raw<'_> {
	type Response = RawResponse;

	fn request_packet_id(&self) -> u8 {
		self.packet_id
	}

	fn request_instruction_id(&self) -> u8 {
		self.instruction_id
	}

	fn request_parameters_len(&self) -> u16 {
		self.instruction_params.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		buffer.copy_from_slice(self.instruction_params);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.packet_id)?;
		crate::InvalidParameterCount::check_max(parameters.len(), self.response_params.len())?;
		self.response_params[..parameters.len()].copy_from_slice(parameters);
		Ok(RawResponse {
			packet_id,
			parameters_len: parameters.len(),
		})
	}
}
