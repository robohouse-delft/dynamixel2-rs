use super::{instruction_id, packet_id, Instruction};

#[derive(Debug, Clone)]
pub struct FactoryReset {
	pub motor_id: u8,
	pub kind: FactoryResetKind,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FactoryResetKind {
	ResetAll = 0xFF,
	KeepId = 0x01,
	KeepIdAndBaudRate = 0x02,
}

impl FactoryReset {
	pub fn unicast(motor_id: u8, kind: FactoryResetKind) -> Self {
		Self { motor_id, kind }
	}

	pub fn broadcast(kind: FactoryResetKind) -> Self {
		Self {
			motor_id: packet_id::BROADCAST,
			kind,
		}
	}
}

impl Instruction for FactoryReset {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::FACTORY_RESET
	}

	fn request_parameters_len(&self) -> u16 {
		1
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		buffer[0] = self.kind as u8;
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;

		Ok(())
	}
}
