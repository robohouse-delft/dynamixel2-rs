use crate::endian::{read_u16_le, write_u16_le};
use crate::instructions::{instruction_id, Instruction};
use crate::{ReadError, TransferError, WriteError};

const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
const HEADER_SIZE: usize = 8;
const STATUS_HEADER_SIZE: usize = 9;

use crate::checksum::calculate_checksum;

pub fn write_instruction<W, I>(stream: &mut W, instruction: &I) -> Result<(), WriteError>
where
	W: std::io::Write + ?Sized,
	I: Instruction,
{
	// Encode the body.
	let raw_body_len: usize = instruction.request_parameters_len().into();

	// Make buffer with enough capacity for fully stuffed message.
	let max_padded_body = crate::bitstuff::maximum_stuffed_len(raw_body_len);
	let mut buffer = vec![0u8; HEADER_SIZE + max_padded_body + 2];

	// Add the header, with a placeholder for the length field.
	buffer[0..4].copy_from_slice(&HEADER_PREFIX);
	buffer[4] = instruction.request_packet_id();
	buffer[7] = instruction.request_instruction_id();
	instruction.encode_request_parameters(&mut buffer[HEADER_SIZE..][..raw_body_len]);

	// Perform bitstuffing on the body.
	// The header never needs stuffing.
	let stuffed_body_len = crate::bitstuff::stuff_inplace(&mut buffer[HEADER_SIZE..], raw_body_len).unwrap();

	write_u16_le(&mut buffer[5..], stuffed_body_len as u16 + 3);

	// Add checksum.
	let checksum_index = HEADER_SIZE + stuffed_body_len;
	let checksum = calculate_checksum(0, &buffer[..checksum_index]);
	write_u16_le(&mut buffer[checksum_index..], checksum);
	buffer.resize(checksum_index + 2, 0);

	// Send message.
	trace!("sending instruction: {:02X?}", buffer);
	stream.write_all(&buffer)?;
	Ok(())
}

pub fn read_response<R, I>(stream: &mut R, instruction: &mut I) -> Result<I::Response, ReadError>
where
	R: std::io::Read + ?Sized,
	I: Instruction,
{
	// TODO: Scan for header prefix rather than assuming it is at start.
	// This matters in case of noise on the serial line.
	let mut raw_header = [0u8; STATUS_HEADER_SIZE];
	stream.read_exact(&mut raw_header[..])?;
	trace!("read status header: {:02X?}", raw_header);

	crate::InvalidHeaderPrefix::check(&raw_header[0..4], HEADER_PREFIX)?;
	crate::InvalidInstruction::check(raw_header[7], instruction_id::STATUS)?;

	let parameters = usize::from(read_u16_le(&raw_header[5..]) - 4);
	let packet_id = raw_header[4];

	let mut body = vec![0u8; parameters + 2];
	stream.read_exact(&mut body)?;
	trace!("read status parameters: {:02X?}", body);
	let checksum_from_msg = read_u16_le(&body[parameters..]);
	let body = &mut body[..parameters];

	let checksum = calculate_checksum(0, &raw_header);
	let checksum = calculate_checksum(checksum, &body);
	crate::InvalidChecksum::check(checksum, checksum_from_msg)?;

	// Remove bit-stuffing on the body.
	let unstuffed_size = crate::bitstuff::unstuff_inplace(body);

	Ok(instruction.decode_response_parameters(packet_id, &body[..unstuffed_size])?)
}

/// Perform a transfer with a single response.
///
/// This is not suitable for broadcast instructions where each motor sends an individual response.
pub fn transfer_single<S, I>(stream: &mut S, instruction: &mut I) -> Result<I::Response, TransferError>
where
	S: std::io::Read + std::io::Write + ?Sized,
	I: Instruction,
{
	write_instruction(stream, instruction)?;
	Ok(read_response(stream, instruction)?)
}
