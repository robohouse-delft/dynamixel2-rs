use super::{id, Instruction};
use crate::endian::{read_u16_le, read_u32_le, write_u16_le};

#[derive(Debug)]
pub struct Read<'a> {
	pub motor_id: u8,
	pub address: u16,
	pub buffer: &'a mut [u8],
}

#[derive(Debug)]
pub struct ReadU8 {
	pub motor_id: u8,
	pub address: u16,
}

#[derive(Debug)]
pub struct ReadU16 {
	pub motor_id: u8,
	pub address: u16,
}

#[derive(Debug)]
pub struct ReadU32 {
	pub motor_id: u8,
	pub address: u16,
}

impl<'a> Read<'a> {
	pub fn new(motor_id: u8, address: u16, buffer: &'a mut [u8]) -> Self {
		Self { motor_id, address, buffer }
	}
}

impl ReadU8 {
	pub fn new(motor_id: u8, address: u16) -> Self {
		Self { motor_id, address}
	}
}

impl ReadU16 {
	pub fn new(motor_id: u8, address: u16) -> Self {
		Self { motor_id, address}
	}
}

impl ReadU32 {
	pub fn new(motor_id: u8, address: u16) -> Self {
		Self { motor_id, address}
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

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), self.buffer.len())?;
		self.buffer.copy_from_slice(parameters);
		Ok(())
	}
}

impl Instruction for ReadU8 {
	type Response = u8;

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
		write_u16_le(&mut buffer[2..], 1);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 1)?;
		Ok(parameters[0])
	}
}

impl Instruction for ReadU16 {
	type Response = u16;

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
		write_u16_le(&mut buffer[2..], 2);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 2)?;
		Ok(read_u16_le(&parameters[0..2]))
	}
}

impl Instruction for ReadU32 {
	type Response = u32;

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
		write_u16_le(&mut buffer[2..], 4);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		crate::InvalidPacketId::check(packet_id, self.motor_id)?;
		crate::InvalidParameterCount::check(parameters.len(), 4)?;
		Ok(read_u32_le(&parameters[0..4]))
	}
}
