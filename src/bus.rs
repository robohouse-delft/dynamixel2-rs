use std::time::{Duration, Instant};

use crate::bytestuff;
use crate::checksum::calculate_checksum;
use crate::endian::{read_u16_le, write_u16_le};
use crate::instructions::{instruction_id, Instruction, Ping, PingResponse};
use crate::{MotorError, ReadError, TransferError, WriteError};

const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
const HEADER_SIZE: usize = 8;
const STATUS_HEADER_SIZE: usize = 9;

/// Dynamixel Protocol 2 communication bus.
pub struct Bus<Stream, ReadBuffer, WriteBuffer> {
	/// The underlying stream (normally a serial port).
	stream: Stream,

	/// The buffer for reading incoming messages.
	read_buffer: ReadBuffer,

	/// The buffer for outgoing messages.
	write_buffer: WriteBuffer,

	/// The total number of valid bytes in the read buffer.
	read_len: usize,
}

impl<Stream> Bus<Stream, Vec<u8>, Vec<u8>>
where
	Stream: std::io::Read + std::io::Write,
{
	/// Create a new bus with 128 byte read and write buffers.
	pub fn new(stream: Stream) -> Self {
		Self::with_buffer_sizes(stream, 128, 128)
	}

	/// Create a new bus with the specified sizes for the read and write buffers.
	pub fn with_buffer_sizes(stream: Stream, read_buffer: usize, write_buffer: usize) -> Self {
		Self::with_buffers(stream, vec![0; read_buffer], vec![0; write_buffer])
	}
}

impl<Stream, ReadBuffer, WriteBuffer> Bus<Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsMut<[u8]>,
	WriteBuffer: AsMut<[u8]>,
{
	/// Create a new bus using pre-allocated buffers.
	pub fn with_buffers(stream: Stream, read_buffer: ReadBuffer, write_buffer: WriteBuffer) -> Self {
		Self {
			stream,
			read_buffer,
			write_buffer,
			read_len: 0,
		}
	}

	/// Write an instruction to a stream, and read a single response.
	///
	/// This is not suitable for broadcast instructions where each motor sends an individual response.
	pub fn transfer_single<I: Instruction>(&mut self, instruction: &mut I, timeout: Duration) -> Result<I::Response, TransferError>
	where
		I: Instruction,
	{
		self.write_instruction(instruction)?;
		Ok(self.read_response(instruction, timeout)?)
	}

	/// Write an instruction to a stream.
	pub fn write_instruction<I: Instruction>(&mut self, instruction: &I) -> Result<(), WriteError> {
		// Encode the body.
		let raw_body_len: usize = instruction.request_parameters_len().into();

		let buffer = self.write_buffer.as_mut();
		if buffer.len() < HEADER_SIZE + raw_body_len + 2 {
			// TODO: return proper error.
			panic!("write buffer not large enough for outgoing mesage");
		}

		// Add the header, with a placeholder for the length field.
		buffer[0..4].copy_from_slice(&HEADER_PREFIX);
		buffer[4] = instruction.request_packet_id();
		buffer[5] = 0;
		buffer[6] = 0;
		buffer[7] = instruction.request_instruction_id();
		instruction.encode_request_parameters(&mut buffer[HEADER_SIZE..][..raw_body_len]);

		// Perform bitstuffing on the body.
		// The header never needs stuffing.
		// TODO: properly propagate error.
		let stuffed_body_len = bytestuff::stuff_inplace(&mut buffer[HEADER_SIZE..], raw_body_len).unwrap();

		write_u16_le(&mut buffer[5..], stuffed_body_len as u16 + 3);

		// Add checksum.
		let checksum_index = HEADER_SIZE + stuffed_body_len;
		let checksum = calculate_checksum(0, &buffer[..checksum_index]);
		write_u16_le(&mut buffer[checksum_index..], checksum);

		// Send message.
		let stuffed_message = &buffer[..checksum_index + 2];
		trace!("sending instruction: {:02X?}", stuffed_message);
		self.stream.write_all(stuffed_message)?;
		Ok(())
	}

	/// Read a response from a stream.
	pub fn read_response<I: Instruction>(&mut self, instruction: &mut I, timeout: Duration) -> Result<I::Response, ReadError> {
		let mut response = self.read_response_(timeout)?;
		trace!("read status packet: {:02X?}", response.buffer());

		let buffer = response.buffer();
		crate::InvalidInstruction::check(buffer[7], instruction_id::STATUS)?;
		let packet_id = buffer[4];

		let checksum_from_msg = read_u16_le(&buffer[buffer.len() - 2..]);

		let checksum = calculate_checksum(0, &buffer[..buffer.len() - 2]);
		crate::InvalidChecksum::check(checksum, checksum_from_msg)?;

		let status = buffer[8];
		if status != 0 {
			return Err(MotorError { raw: status }.into());
		}

		// Remove bit-stuffing on the body.
		let parameters_end = buffer.len() - 2;
		let parameters = &mut buffer[STATUS_HEADER_SIZE..parameters_end];
		let unstuffed_size = bytestuff::unstuff_inplace(parameters);

		Ok(instruction.decode_response_parameters(packet_id, &parameters[..unstuffed_size])?)
	}

	/// Scan a bus for motors with a broadcast ping, calling an [`FnMut`] for each response.
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are passed to the handler.
	pub fn scan<F>(&mut self, mut on_response: F) -> Result<(), WriteError>
	where
		F: FnMut(Result<PingResponse, ReadError>),
	{
		let mut ping = Ping::broadcast();
		self.write_instruction(&ping)?;

		// TODO: See if we can terminate quicker.
		// Peek at the official SDK to see what they do.

		for _ in 0..253 {
			let response = self.read_response(&mut ping, Duration::from_millis(100));
			if let Err(ReadError::Io(e)) = &response {
				if e.kind() == std::io::ErrorKind::TimedOut {
					continue;
				}
			}
			on_response(response)
		}

		Ok(())
	}

	/// Scan a bus for motors with a broadcast ping, returning the responses in a [`Vec`].
	///
	/// Only timeouts are filtered out since they indicate a lack of response.
	/// All other responses (including errors) are collected.
	pub fn scan_to_vec(&mut self) -> Result<Vec<Result<PingResponse, ReadError>>, WriteError> {
		let mut result = Vec::with_capacity(253);
		self.scan(|x| result.push(x))?;
		Ok(result)
	}

	/// Read a raw response from the stream.
	fn read_response_(&mut self, timeout: Duration) -> std::io::Result<Response<Stream, ReadBuffer, WriteBuffer>> {
		let response_len;
		let deadline = Instant::now() + timeout;
		while Instant::now() <= deadline {
			// Try to read more data from the buffer.
			let new_data = self.stream.read(&mut self.read_buffer.as_mut()[self.read_len..])?;
			if new_data == 0 {
				return Err(std::io::ErrorKind::TimedOut.into());
			}

			self.read_len += new_data;
			self.remove_garbage();

			let read_buffer = &self.read_buffer.as_mut()[..self.read_len];
			if !read_buffer.starts_with(&HEADER_PREFIX) {
				continue;
			}

			if self.read_len < STATUS_HEADER_SIZE {
				continue;
			}

			let body_len = read_buffer[5] as usize + read_buffer[6] as usize * 256;
			let body_len = body_len - 2; // Length includes instruction and error fields, which is already included in STATUS_HEADER_SIZE too.

			if self.read_len >= STATUS_HEADER_SIZE + body_len {
				response_len = STATUS_HEADER_SIZE + body_len;
				return Ok(Response {
					stream: self,
					len: response_len,
				});
			}
		}

		Err(std::io::ErrorKind::TimedOut.into())
	}

	/// Remove leading garbage data from the read buffer.
	fn remove_garbage(&mut self) {
		let read_buffer = self.read_buffer.as_mut();
		let garbage_len = find_header(&read_buffer[..self.read_len]);
		read_buffer.copy_within(garbage_len..self.read_len, 0);
		self.read_len -= garbage_len;
	}
}

/// A response that is currently in the read buffer of a bus.
struct Response<'a, Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsMut<[u8]>,
	WriteBuffer: AsMut<[u8]>,
{
	stream: &'a mut Bus<Stream, ReadBuffer, WriteBuffer>,
	len: usize,
}


impl<'a, Stream, ReadBuffer, WriteBuffer> Response<'a, Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsMut<[u8]>,
	WriteBuffer: AsMut<[u8]>,
{
	fn buffer(&mut self) -> &mut [u8] {
		&mut self.stream.read_buffer.as_mut()[..self.len]
	}
}

impl<'a, Stream, ReadBuffer, WriteBuffer> Drop for Response<'a, Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsMut<[u8]>,
	WriteBuffer: AsMut<[u8]>,
{
	fn drop(&mut self) {
		let read_buffer = self.stream.read_buffer.as_mut();
		read_buffer.copy_within(self.len..self.stream.read_len, 0);
		self.stream.read_len -= self.len;
	}
}

/// Find the potential starting position of a header.
///
/// This will return the first possible position of the header prefix.
/// Note that if the buffer ends with a partial header prefix,
/// the start position of the partial header prefix is returned.
fn find_header(buffer: &[u8]) -> usize {
	for i in 0..buffer.len() {
		let possible_prefix = HEADER_PREFIX.len().min(buffer.len() - i);
		if buffer[i..].starts_with(&HEADER_PREFIX[..possible_prefix]) {
			return i;
		}
	}

	buffer.len()
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_find_garbage_end() {
		assert!(find_header(&[0xFF]) == 0);
		assert!(find_header(&[0xFF, 0xFF]) == 0);
		assert!(find_header(&[0xFF, 0xFF, 0xFD]) == 0);
		assert!(find_header(&[0xFF, 0xFF, 0xFD, 0x00]) == 0);
		assert!(find_header(&[0xFF, 0xFF, 0xFD, 0x00, 9]) == 0);

		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF]) == 5);
		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF, 0xFF]) == 5);
		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF, 0xFF, 0xFD]) == 5);
		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF, 0xFF, 0xFD, 0x00]) == 5);
		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF, 0xFF, 0xFD, 0x00, 9]) == 5);

		assert!(find_header(&[0xFF, 1]) == 2);
		assert!(find_header(&[0, 1, 2, 3, 4, 0xFF, 6]) == 7);
	}
}
