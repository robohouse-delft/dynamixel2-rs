use serial2::SerialPort;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::bytestuff;
use crate::checksum::calculate_checksum;
use crate::endian::{read_u16_le, read_u32_le, read_u8_le, write_u16_le};
use crate::{ReadError, TransferError, WriteError};

const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];
const HEADER_SIZE: usize = 8;
const STATUS_HEADER_SIZE: usize = 9;

/// Dynamixel Protocol 2 communication bus.
pub struct Bus<ReadBuffer, WriteBuffer> {
	/// The underlying stream (normally a serial port).
	serial_port: SerialPort,

	/// The timeout for reading a single response.
	read_timeout: Duration,

	/// The buffer for reading incoming messages.
	read_buffer: ReadBuffer,

	/// The total number of valid bytes in the read buffer.
	read_len: usize,

	/// The buffer for outgoing messages.
	write_buffer: WriteBuffer,
}

impl Bus<Vec<u8>, Vec<u8>> {
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::open_with_buffers()`] if you want to use a custom buffers.
	pub fn open(path: impl AsRef<Path>, baud_rate: u32, read_timeout: Duration) -> std::io::Result<Self> {
		let port = SerialPort::open(path, baud_rate)?;
		Ok(Self::new(port, read_timeout))
	}

	/// Create a new bus for an open serial port.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	/// Use [`Self::with_buffers()`] if you want to use a custom buffers.
	pub fn new(stream: SerialPort, read_timeout: Duration) -> Self {
		Self::with_buffers(stream, read_timeout, vec![0; 128], vec![0; 128])
	}
}

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Open a serial port with the given baud rate.
	///
	/// This will allocate a new read and write buffer of 128 bytes each.
	pub fn open_with_buffers(
		path: impl AsRef<Path>,
		baud_rate: u32,
		read_timeout: Duration,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> std::io::Result<Self> {
		let port = SerialPort::open(path, baud_rate)?;
		Ok(Self::with_buffers(port, read_timeout, read_buffer, write_buffer))
	}

	/// Create a new bus using pre-allocated buffers.
	pub fn with_buffers(serial_port: SerialPort, read_timeout: Duration, read_buffer: ReadBuffer, mut write_buffer: WriteBuffer) -> Self {
		// Pre-fill write buffer with the header prefix.
		assert!(write_buffer.as_mut().len() >= HEADER_SIZE + 2);
		write_buffer.as_mut()[..4].copy_from_slice(&HEADER_PREFIX);

		Self {
			serial_port,
			read_timeout,
			read_buffer,
			read_len: 0,
			write_buffer,
		}
	}

	/// Write a raw instruction to a stream, and read a single raw response.
	///
	/// This function also checks that the packet ID of the status response matches the one from the instruction.
	///
	/// This is not suitable for broadcast instructions.
	/// For broadcast instructions, each motor sends an individual response or no response is send at all.
	/// Instead, use [`Self::write_instruction`] and [`Self::read_status_response`].
	pub fn transfer_single<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<StatusPacket<ReadBuffer, WriteBuffer>, TransferError>
	where
		F: FnOnce(&mut [u8]),
	{
		self.write_instruction(packet_id, instruction_id, parameter_count, encode_parameters)?;
		let response = self.read_status_response()?;
		crate::error::InvalidPacketId::check(response.packet_id(), packet_id).map_err(crate::ReadError::from)?;
		Ok(response)
	}

	/// Write an instruction message to the bus.
	pub fn write_instruction<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError>
	where
		F: FnOnce(&mut [u8]),
	{
		// Throw away old data in the read buffer.
		// Ideally, we would also flush the kernel buffer, but the serial crate doesn't expose that.
		// We don't do this when reading a reply, because we might multiple replies for one instruction,
		// and read() can potentially read more than one reply per syscall.
		self.read_len = 0;

		let buffer = self.write_buffer.as_mut();
		if buffer.len() < HEADER_SIZE + parameter_count + 2 {
			// TODO: return proper error.
			panic!("write buffer not large enough for outgoing mesage");
		}

		// Add the header, with a placeholder for the length field.
		buffer[4] = packet_id;
		buffer[5] = 0;
		buffer[6] = 0;
		buffer[7] = instruction_id;
		encode_parameters(&mut buffer[HEADER_SIZE..][..parameter_count]);

		// Perform bitstuffing on the body.
		// The header never needs stuffing.
		// TODO: properly propagate error.
		let stuffed_body_len = bytestuff::stuff_inplace(&mut buffer[HEADER_SIZE..], parameter_count).unwrap();

		write_u16_le(&mut buffer[5..], stuffed_body_len as u16 + 3);

		// Add checksum.
		let checksum_index = HEADER_SIZE + stuffed_body_len;
		let checksum = calculate_checksum(0, &buffer[..checksum_index]);
		write_u16_le(&mut buffer[checksum_index..], checksum);

		// Send message.
		let stuffed_message = &buffer[..checksum_index + 2];
		trace!("sending instruction: {:02X?}", stuffed_message);
		self.serial_port.discard_input_buffer().map_err(WriteError::DiscardBuffer)?;
		self.serial_port.write_all(stuffed_message).map_err(WriteError::Write)?;
		Ok(())
	}

	/// Read a raw status response from the bus.
	pub fn read_status_response(&mut self) -> Result<StatusPacket<ReadBuffer, WriteBuffer>, ReadError> {
		let deadline = Instant::now() + self.read_timeout;
		let stuffed_message_len = loop {
			self.remove_garbage();

			// The call to remove_garbage() removes all leading bytes that don't match a status header.
			// So if there's enough bytes left, it's a status header.
			if self.read_len > STATUS_HEADER_SIZE {
				let read_buffer = &self.read_buffer.as_mut()[..self.read_len];
				let body_len = read_buffer[5] as usize + read_buffer[6] as usize * 256;
				let body_len = body_len - 2; // Length includes instruction and error fields, which is already included in STATUS_HEADER_SIZE too.

				if self.read_len >= STATUS_HEADER_SIZE + body_len {
					break STATUS_HEADER_SIZE + body_len;
				}
			}

			if Instant::now() > deadline {
				trace!(
					"timeout reading status response, data in buffer: {:02X?}",
					&self.read_buffer.as_ref()[..self.read_len]
				);
				return Err(std::io::ErrorKind::TimedOut.into());
			}

			// Try to read more data into the buffer.
			let new_data = self.serial_port.read(&mut self.read_buffer.as_mut()[self.read_len..])?;
			if new_data == 0 {
				continue;
			}

			self.read_len += new_data;
		};

		let buffer = self.read_buffer.as_mut();
		let parameters_end = stuffed_message_len - 2;
		trace!("read packet: {:02X?}", &buffer[..parameters_end]);

		let checksum_message = read_u16_le(&buffer[parameters_end..]);
		let checksum_computed = calculate_checksum(0, &buffer[..parameters_end]);
		if checksum_message != checksum_computed {
			self.consume_read_bytes(stuffed_message_len);
			return Err(crate::InvalidChecksum {
				message: checksum_message,
				computed: checksum_computed,
			}
			.into());
		}

		// Remove byte-stuffing from the parameters.
		let parameter_count = bytestuff::unstuff_inplace(&mut buffer[STATUS_HEADER_SIZE..parameters_end]);

		// Creating the status packet struct here means that the data gets purged from the buffer even if we return early using the try operator.
		let response = StatusPacket {
			bus: self,
			stuffed_message_len,
			parameter_count,
		};

		crate::InvalidInstruction::check(response.instruction_id(), crate::instructions::instruction_id::STATUS)?;
		crate::MotorError::check(response.error())?;
		Ok(response)
	}
}

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Remove leading garbage data from the read buffer.
	fn remove_garbage(&mut self) {
		let read_buffer = self.read_buffer.as_mut();
		let garbage_len = find_header(&read_buffer[..self.read_len]);
		if garbage_len > 0 {
			debug!("skipping {} bytes of leading garbage.", garbage_len);
			trace!("skipped garbage: {:02X?}", &read_buffer[..garbage_len]);
		}
		self.consume_read_bytes(garbage_len);
	}

	fn consume_read_bytes(&mut self, len: usize) {
		debug_assert!(len <= self.read_len);
		self.read_buffer.as_mut().copy_within(len..self.read_len, 0);
		self.read_len -= len;
	}
}

/// A status response that is currently in the read buffer of a bus.
///
/// When dropped, the response data is removed from the read buffer.
pub struct StatusPacket<'a, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// The bus that read the message.
	bus: &'a mut Bus<ReadBuffer, WriteBuffer>,

	/// The total length of the stuffed message.
	stuffed_message_len: usize,

	/// The number of parameters after removing byte-stuffing.
	parameter_count: usize,
}

impl<'a, ReadBuffer, WriteBuffer> StatusPacket<'a, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the raw bytes of the message.
	///
	/// This includes the message header and the parameters.
	/// It does not include the CRC or byte-stuffing.
	pub fn as_bytes(&self) -> &[u8] {
		&self.bus.read_buffer.as_ref()[..STATUS_HEADER_SIZE + self.parameter_count]
	}

	/// The packet ID of the response.
	pub fn packet_id(&self) -> u8 {
		self.as_bytes()[4]
	}

	/// The instruction ID of the response.
	pub fn instruction_id(&self) -> u8 {
		self.as_bytes()[7]
	}

	/// The error field of the response.
	pub fn error(&self) -> u8 {
		self.as_bytes()[8]
	}

	/// The error number of the status packet.
	///
	/// This is the lower 7 bits of the error field.
	pub fn error_number(&self) -> u8 {
		self.error() & !0x80
	}

	/// The alert bit from the error field of the response.
	///
	/// This is the 8th bit of the error field.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub fn alert(&self) -> bool {
		self.error() & 0x80 != 0
	}

	/// The parameters of the response.
	pub fn parameters(&self) -> &[u8] {
		&self.as_bytes()[STATUS_HEADER_SIZE..][..self.parameter_count]
	}
}

impl<'a, ReadBuffer, WriteBuffer> Drop for StatusPacket<'a, ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		self.bus.consume_read_bytes(self.stuffed_message_len);
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

/// A response from a motor.
#[derive(Debug)]
pub struct Response<T> {
	/// The motor that sent the response.
	pub motor_id: u8,

	/// The alert bit from the response message.
	///
	/// If this bit is set, you can normally check the "Hardware Error" register for more details.
	/// Consult the manual of your motor for more information.
	pub alert: bool,

	/// The data from the motor.
	pub data: T,
}

impl<'a, ReadBuffer, WriteBuffer> TryFrom<StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<()>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameter_count, 0)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: (),
		})
	}
}

impl<'a, 'b, ReadBuffer, WriteBuffer> From<&'b StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<&'b [u8]>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn from(status_packet: &'b StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Self {
		Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: status_packet.parameters(),
		}
	}
}

impl<'a, ReadBuffer, WriteBuffer> From<StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<Vec<u8>>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Self {
		Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: status_packet.parameters().to_owned(),
		}
	}
}

impl<'a, ReadBuffer, WriteBuffer> TryFrom<StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<u8>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameter_count, 1)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u8_le(status_packet.parameters()),
		})
	}
}

impl<'a, ReadBuffer, WriteBuffer> TryFrom<StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<u16>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameter_count, 2)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u16_le(status_packet.parameters()),
		})
	}
}

impl<'a, ReadBuffer, WriteBuffer> TryFrom<StatusPacket<'a, ReadBuffer, WriteBuffer>> for Response<u32>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Error = crate::InvalidParameterCount;

	fn try_from(status_packet: StatusPacket<'a, ReadBuffer, WriteBuffer>) -> Result<Self, Self::Error> {
		crate::InvalidParameterCount::check(status_packet.parameter_count, 4)?;
		Ok(Self {
			motor_id: status_packet.packet_id(),
			alert: status_packet.alert(),
			data: read_u32_le(status_packet.parameters()),
		})
	}
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
