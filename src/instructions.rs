use byteorder::ByteOrder;
use byteorder::LittleEndian as LE;

use crate::crc::calculate_crc;

pub const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
pub const HEADER_SIZE: usize = 8;

pub mod id {
	pub const PING:  u8 = 1;
	pub const READ:  u8 = 2;
	pub const WRITE: u8 = 3;
	pub const REG_WRITE: u8 = 4;
	pub const ACTION: u8 = 5;
	pub const FACTORY_RESET: u8 = 6;
	pub const REBOOT: u8 = 7;
	pub const CLEAR: u8 = 8;
	pub const SYNC_READ: u8 = 9;
	pub const SYNC_WRITE: u8 = 10;
	pub const BULK_READ: u8 = 11;
	pub const BULK_WRITE: u8 = 12;
	pub const STATUS: u8 = 0x55;
}

pub trait Instruction {
	fn packet_id(&self) -> u8;

	fn instruction_id(&self) -> u8;

	// The length of parameter bytes, unstuffed body.
	fn parameters_len(&self) -> u16;

	fn encode_body_to(&self, buffer: &mut [u8]);
}

pub trait Status: Sized {
	fn decode_body_from(packet_id: u8, body: &[u8]) -> Result<Self, ReadError>;
}

#[derive(Debug)]
pub enum ReadError {
	Io(std::io::Error),
	InvalidPrefix,
	InvalidCrc,
	InvalidPacketId,
	InvalidInstruction,
	InvalidParameterCount,
}

impl From<std::io::Error> for ReadError {
	fn from(other: std::io::Error) -> Self {
		Self::Io(other)
	}
}

pub fn write_instruction<W, I>(stream: &mut W, instruction: &I) -> std::io::Result<()>
where
	W: std::io::Write + ?Sized,
	I: Instruction,
{
	// Encode the body.
	let raw_body_len : usize = instruction.parameters_len().into();

	// Make buffer with enough capacity for fully stuffed message.
	let max_padded_body = crate::bitstuff::maximum_stuffed_len(raw_body_len);
	let mut buffer = vec![0u8; HEADER_SIZE + max_padded_body + 2];

	// Add the header, with a placeholder for the length field.
	buffer[0..4].copy_from_slice(&HEADER_PREFIX);
	buffer[4] = instruction.packet_id();
	buffer[7] = instruction.instruction_id();
	instruction.encode_body_to(&mut buffer[HEADER_SIZE..][..raw_body_len]);

	// Perform bitstuffing on the body.
	// The header never needs stuffing.
	let stuffed_body_len = crate::bitstuff::stuff_inplace(&mut buffer[HEADER_SIZE..], raw_body_len).unwrap();

	buffer[5] = (stuffed_body_len >> 0 & 0xFF) as u8;
	buffer[6] = (stuffed_body_len >> 8 & 0xFF) as u8;

	// Add checksum.
	let crc_index = HEADER_SIZE + stuffed_body_len;
	let checksum = calculate_crc(0, &buffer[..crc_index]);
	buffer[crc_index + 0] = (checksum >> 0 & 0xFF) as u8;
	buffer[crc_index + 1] = (checksum >> 8 & 0xFF) as u8;
	buffer.resize(crc_index + 2, 0);

	// Send message.
	trace!("Sending instruction: {:02X?}", buffer);
	stream.write_all(&buffer)
}

pub fn read_status<R, S>(stream: &mut R) -> Result<S, ReadError>
where
	R: std::io::Read + ?Sized,
	S: Status,
{
	let mut raw_header = [0u8; 9];
	stream.read_exact(&mut raw_header[..])?;
	trace!("Read status header: {:02X?}", raw_header);

	if &raw_header[0..4] != HEADER_PREFIX {
		return Err(ReadError::InvalidPrefix);
	}

	if raw_header[7] != id::STATUS {
		return Err(ReadError::InvalidInstruction);
	}

	let parameters = usize::from(LE::read_u16(&raw_header[5..7]) - 4);
	let packet_id = raw_header[4];

	let mut body = vec![0u8; parameters + 2];
	stream.read_exact(&mut body)?;
	trace!("Read status body: {:02X?}", body);
	let crc_from_msg = 0
		| (body.pop().unwrap() as u16) << 8
		| (body.pop().unwrap() as u16) << 0;

	let crc = calculate_crc(0, &raw_header);
	let crc = calculate_crc(crc, &body);
	if crc != crc_from_msg {
		return Err(ReadError::InvalidCrc)
	}

	// Remove bit-stuffing on the body.
	crate::bitstuff::unstuff_inplace_vec(&mut body);

	S::decode_body_from(packet_id, &body)
}

#[derive(Debug, Clone)]
pub struct PingInstruction {
	packet_id: u8,
}

impl PingInstruction {
	pub fn new(motor_id: u8) -> Self {
		Self {
			packet_id: motor_id,
		}
	}
}

impl Instruction for PingInstruction {
	fn packet_id(&self) -> u8 {
		self.packet_id
	}

	fn instruction_id(&self) -> u8 {
		id::PING
	}

	fn parameters_len(&self) -> u16 {
		0
	}

	fn encode_body_to(&self, _buffer: &mut [u8]) {
	}
}

#[derive(Debug, Clone)]
pub struct PingStatus {
	pub packet_id: u8,
	pub model: u16,
	pub firmware: u8,
}

impl Status for PingStatus {
	fn decode_body_from(packet_id: u8, body: &[u8]) -> Result<Self, ReadError> {
		if body.len() != 3 {
			return Err(ReadError::InvalidParameterCount);
		}

		Ok(Self {
			packet_id,
			model: LE::read_u16(&body[0..2]),
			firmware: body[2],
		})
	}
}
