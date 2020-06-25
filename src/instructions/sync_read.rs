use super::{instruction_id, packet_id, Instruction};
use crate::endian::{read_u16_le, read_u32_le, write_u16_le};

/// Perform a sync read, reading the data into a supplied buffer.
#[derive(Debug)]
pub struct SyncRead<'a> {
	/// The motors ro read from.
	pub motor_ids: &'a [u8],

	/// The address to read from.
	pub address: u16,

	/// The buffer to read into.
	buffer: &'a mut [u8],
}

/// Perform a sync read, returning the data as [`Vec`].
#[derive(Debug)]
pub struct SyncReadVec<'a> {
	pub motor_ids: &'a [u8],
	pub address: u16,
	pub length: u16,
}

/// Synchronously read an [`u8`] from multiple motors.
#[derive(Debug)]
pub struct SyncReadU8<'a> {
	pub motor_ids: &'a [u8],
	pub address: u16,
}

/// Synchronously read an [`u16`] from multiple motors.
#[derive(Debug)]
pub struct SyncReadU16<'a> {
	pub motor_ids: &'a [u8],
	pub address: u16,
}

/// Synchronously read an [`u32`] from multiple motors.
#[derive(Debug)]
pub struct SyncReadU32<'a> {
	pub motor_ids: &'a [u8],
	pub address: u16,
}

impl<'a> SyncRead<'a> {
	/// Create a new SyncRead instruction to read data from a list motors.
	///
	/// To read 12 bytes from 3 motors, you must supply a buffer of 36 bytes.
	///
	/// # Panic
	/// Panics if the buffer size is not a multiple of the number of motors.
	pub fn new(motor_ids: &'a [u8], address: u16, buffer: &'a mut [u8]) -> Self {
		if buffer.len() % motor_ids.len() != 0 {
			panic!("invalid buffer size: buffer size ({}) must be a multiple of the number of motors ({})", buffer.len(), motor_ids.len());
		}
		Self { motor_ids, address, buffer }
	}

	/// Get a non-mutable reference to the read buffer.
	pub fn buffer(&self) -> &[u8] {
		self.buffer
	}

	/// Get a mutable reference to the read buffer.
	pub fn buffer_mut(&mut self) -> &mut [u8] {
		self.buffer
	}

	/// The number of bytes to read from each motor.
	pub fn length(&self) -> usize {
		self.buffer.len() / self.motor_ids.len()
	}
}

impl<'a> SyncReadVec<'a> {
	/// Create a new SyncReadVec instruction to read data from a list of motors.
	pub fn new(motor_ids: &'a [u8], address: u16, length: u16) -> Self {
		Self { motor_ids, address, length }
	}
}

impl<'a> SyncReadU8<'a> {
	/// Create a new SyncReadVec instruction to read data from a list of motors.
	pub fn new(motor_ids: &'a [u8], address: u16) -> Self {
		Self { motor_ids, address }
	}
}

impl<'a> SyncReadU16<'a> {
	/// Create a new SyncReadVec instruction to read data from a list of motors.
	pub fn new(motor_ids: &'a [u8], address: u16) -> Self {
		Self { motor_ids, address }
	}
}

impl<'a> SyncReadU32<'a> {
	/// Create a new SyncReadVec instruction to read data from a list of motors.
	pub fn new(motor_ids: &'a [u8], address: u16) -> Self {
		Self { motor_ids, address }
	}
}

impl Instruction for SyncRead<'_> {
	type Response = u8;

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_READ
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], self.length() as u16);
		buffer[4..].copy_from_slice(self.motor_ids);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		let length = self.length();
		let index = self.motor_ids.iter()
			.position(|&x| x == packet_id)
			.ok_or_else(|| crate::InvalidPacketId { actual: packet_id, expected: None })?;
		crate::InvalidParameterCount::check(parameters.len(), length)?;

		self.buffer[index * length..][..length].copy_from_slice(parameters);
		Ok(packet_id)
	}
}

impl Instruction for SyncReadVec<'_> {
	type Response = (u8, Vec<u8>);

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_READ
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], self.length);
		buffer[4..].copy_from_slice(self.motor_ids);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		if !self.motor_ids.contains(&packet_id) {
			return Err(crate::InvalidPacketId { actual: packet_id, expected: None }.into());
		}
		crate::InvalidParameterCount::check(parameters.len(), self.length.into())?;
		Ok((packet_id, parameters.into()))
	}
}

impl Instruction for SyncReadU8<'_> {
	type Response = (u8, u8);

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_READ
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 1);
		buffer[4..].copy_from_slice(self.motor_ids);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		if !self.motor_ids.contains(&packet_id) {
			return Err(crate::InvalidPacketId { actual: packet_id, expected: None }.into());
		}
		crate::InvalidParameterCount::check(parameters.len(), 1)?;
		Ok((packet_id, parameters[0]))
	}
}

impl Instruction for SyncReadU16<'_> {
	type Response = (u8, u16);

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_READ
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 2);
		buffer[4..].copy_from_slice(self.motor_ids);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		if !self.motor_ids.contains(&packet_id) {
			return Err(crate::InvalidPacketId { actual: packet_id, expected: None }.into());
		}
		crate::InvalidParameterCount::check(parameters.len(), 2)?;
		Ok((packet_id, read_u16_le(parameters)))
	}
}

impl Instruction for SyncReadU32<'_> {
	type Response = (u8, u32);

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_READ
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 4);
		buffer[4..].copy_from_slice(self.motor_ids);
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		if !self.motor_ids.contains(&packet_id) {
			return Err(crate::InvalidPacketId { actual: packet_id, expected: None }.into());
		}
		crate::InvalidParameterCount::check(parameters.len(), 4)?;
		Ok((packet_id, read_u32_le(parameters)))
	}
}
