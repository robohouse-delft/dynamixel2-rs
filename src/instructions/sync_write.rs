use super::{instruction_id, packet_id, Instruction};
use crate::endian::{write_u16_le, write_u32_le};

/// Perform a sync write using a supplied buffer.
#[derive(Debug)]
pub struct SyncWrite<'a> {
	/// The motors to write to.
	pub motor_ids: &'a [u8],

	/// The address to write to.
	pub address: u16,

	/// The buffer to write.
	buffer: &'a [u8],
}

/// A motor ID and data to write.
#[derive(Debug)]
pub struct WriteData<T> {
	/// The ID of the motor to write to.
	pub motor_id: u8,

	/// The data to write to the motor.
	pub data: T,
}

impl<T> WriteData<T> {
	/// Create a new [`WriteData`] struct.
	pub fn new(motor_id: u8, data: T) -> Self {
		Self { motor_id, data }
	}
}

/// Synchronously write an [`u8`] to multiple motors.
#[derive(Debug)]
pub struct SyncWriteU8<'a> {
	/// The address to write to.
	pub address: u16,

	/// The data to write, including motor IDs to write to.
	pub data: &'a [WriteData<u8>]
}

/// Synchronously write an [`u16`] to multiple motors.
#[derive(Debug)]
pub struct SyncWriteU16<'a> {
	/// The address to write to.
	pub address: u16,

	/// The data to write, including motor IDs to write to.
	pub data: &'a [WriteData<u16>]
}

/// Synchronously write an [`u32`] to multiple motors.
#[derive(Debug)]
pub struct SyncWriteU32<'a> {
	/// The address to write to.
	pub address: u16,

	/// The data to write, including motor IDs to write to.
	pub data: &'a [WriteData<u32>]
}

impl<'a> SyncWrite<'a> {
	/// Create a new SyncWrite instruction to write data to a list motors.
	///
	/// To write 12 bytes from 3 motors, you must supply a buffer of 36 bytes.
	///
	/// # Panic
	/// Panics if the buffer size is not a multiple of the number of motors.
	pub fn new(motor_ids: &'a [u8], address: u16, buffer: &'a [u8]) -> Self {
		if buffer.len() % motor_ids.len() != 0 {
			panic!("invalid buffer size: buffer size ({}) must be a multiple of the number of motors ({})", buffer.len(), motor_ids.len());
		}
		Self { motor_ids, address, buffer }
	}

	/// Get a non-mutable reference to the write buffer.
	pub fn buffer(&self) -> &[u8] {
		self.buffer
	}

	/// The number of bytes to write to each motor.
	pub fn length(&self) -> usize {
		self.buffer.len() / self.motor_ids.len()
	}
}

impl<'a> SyncWriteU8<'a> {
	/// Create a new SyncWriteU8 instruction to write an [`u8`] to a list of motors.
	pub fn new(address: u16, data: &'a [WriteData<u8>]) -> Self {
		Self { address, data }
	}
}

impl<'a> SyncWriteU16<'a> {
	/// Create a new SyncWriteU8 instruction to write an [`u16`] to a list of motors.
	pub fn new(address: u16, data: &'a [WriteData<u16>]) -> Self {
		Self { address, data }
	}
}

impl<'a> SyncWriteU32<'a> {
	/// Create a new SyncWriteU8 instruction to write an [`u32`] to a list of motors.
	pub fn new(address: u16, data: &'a [WriteData<u32>]) -> Self {
		Self { address, data }
	}
}

impl Instruction for SyncWrite<'_> {
	type Response = u8;

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.motor_ids.len() as u16 * (self.length() as u16 + 1)
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], self.length() as u16);
		let length = self.length();
		let stride = length + 1;
		for (i, &motor_id) in self.motor_ids.iter().enumerate() {
			let start = 4 + i * stride;
			buffer[start] = motor_id;
			buffer[start + 1..][..length].copy_from_slice(&self.buffer[i * length..][..length]);
		}
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		self.motor_ids.iter()
			.position(|&x| x == packet_id)
			.ok_or_else(|| crate::InvalidPacketId { actual: packet_id, expected: None })?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;

		Ok(packet_id)
	}
}

impl Instruction for SyncWriteU8<'_> {
	type Response = u8;

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.data.len() as u16 * 2
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 1);
		for (i, entry) in self.data.iter().enumerate() {
			buffer[i * 2 + 4] = entry.motor_id;
			buffer[i * 2 + 5] = entry.data;
		}
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		self.data
			.iter()
			.position(|x| x.motor_id == packet_id)
			.ok_or_else(|| crate::InvalidPacketId { actual: packet_id, expected: None })?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(packet_id)
	}
}

impl Instruction for SyncWriteU16<'_> {
	type Response = u8;

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.data.len() as u16 * 3
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 1);
		for (i, entry) in self.data.iter().enumerate() {
			buffer[i * 3 + 4] = entry.motor_id;
			write_u16_le(&mut buffer[i * 3 + 5..], entry.data);
		}
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		self.data
			.iter()
			.position(|x| x.motor_id == packet_id)
			.ok_or_else(|| crate::InvalidPacketId { actual: packet_id, expected: None })?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(packet_id)
	}
}

impl Instruction for SyncWriteU32<'_> {
	type Response = u8;

	fn request_packet_id(&self) -> u8 {
		packet_id::BROADCAST
	}

	fn request_instruction_id(&self) -> u8 {
		instruction_id::SYNC_WRITE
	}

	fn request_parameters_len(&self) -> u16 {
		4 + self.data.len() as u16 * 5
	}

	fn encode_request_parameters(&self, buffer: &mut [u8]) {
		write_u16_le(&mut buffer[0..2], self.address);
		write_u16_le(&mut buffer[2..4], 1);
		for (i, entry) in self.data.iter().enumerate() {
			buffer[i * 5 + 4] = entry.motor_id;
			write_u32_le(&mut buffer[i * 5 + 5..], entry.data);
		}
	}

	fn decode_response_parameters(&mut self, packet_id: u8, parameters: &[u8]) -> Result<Self::Response, crate::InvalidMessage> {
		self.data
			.iter()
			.position(|x| x.motor_id == packet_id)
			.ok_or_else(|| crate::InvalidPacketId { actual: packet_id, expected: None })?;
		crate::InvalidParameterCount::check(parameters.len(), 0)?;
		Ok(packet_id)
	}
}
