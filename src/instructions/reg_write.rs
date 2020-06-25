use super::{instruction_id, Instruction};
use crate::endian::{write_u16_le, write_u32_le};

#[derive(Debug, Clone)]
pub struct RegWrite<'a> {
	pub motor_id: u8,
	pub address: u16,
	pub data: &'a [u8],
}

#[derive(Debug, Clone)]
pub struct RegWriteU8 {
	pub motor_id: u8,
	pub address: u16,
	pub data: u8,
}

#[derive(Debug, Clone)]
pub struct RegWriteU16 {
	pub motor_id: u8,
	pub address: u16,
	pub data: u16,
}

#[derive(Debug, Clone)]
pub struct RegWriteU32 {
	pub motor_id: u8,
	pub address: u16,
	pub data: u32,
}

impl<'a> RegWrite<'a> {
	pub fn new(motor_id: u8, address: u16, data: &'a [u8]) -> Self {
		Self { motor_id, address, data }
	}
}

impl RegWriteU8 {
	pub fn new(motor_id: u8, address: u16, data: u8) -> Self {
		Self { motor_id, address, data }
	}
}

impl RegWriteU16 {
	pub fn new(motor_id: u8, address: u16, data: u16) -> Self {
		Self { motor_id, address, data }
	}
}

impl RegWriteU32 {
	pub fn new(motor_id: u8, address: u16, data: u32) -> Self {
		Self { motor_id, address, data }
	}
}

impl Instruction for RegWrite<'_> {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::REG_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		2 + self.data.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..], self.address);
		buffer[2..].copy_from_slice(&self.data);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}

impl Instruction for RegWriteU8 {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::REG_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		3
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		buffer[2] = self.data;
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}

impl Instruction for RegWriteU16 {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::REG_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		4
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], self.data);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}

impl Instruction for RegWriteU32 {
	type Response = ();

	fn request_packet_id(&self) -> u8 {
		self.motor_id
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::REG_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		6
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u32_le(&mut buffer[2..6], self.data);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check_ignore_broadcast(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(())
	}
}
