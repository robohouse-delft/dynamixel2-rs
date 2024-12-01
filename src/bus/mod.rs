//! Low level interface to a DYNAMIXEL Protocol 2.0 bus.

use crate::checksum::calculate_checksum;
use crate::endian::{read_u16_le, write_u16_le};
use crate::packet::{Packet, HEADER_PREFIX, INSTRUCTION_HEADER_SIZE, STATUS_HEADER_SIZE};
use crate::{bytestuff, ReadError, SerialPort, WriteError};
use core::time::Duration;

mod status_packet;
pub use status_packet::*;

/// Low level interface to a DYNAMIXEL Protocol 2.0 bus.
///
/// Does not assume anything about the direction of communication.
/// Used by [`crate::Client`] and [`crate::Device`].
pub(crate) struct Bus<ReadBuffer, WriteBuffer, T> {
	/// The underlying stream (normally a serial port).
	pub(crate) serial_port: T,

	/// The baud rate of the serial port, if known.
	pub(crate) baud_rate: u32,

	/// The buffer for reading incoming messages.
	pub(crate) read_buffer: ReadBuffer,

	/// The total number of valid bytes in the read buffer.
	pub(crate) read_len: usize,

	/// The number of leading bytes in the read buffer that have already been used.
	pub(crate) used_bytes: usize,

	/// The buffer for outgoing messages.
	pub(crate) write_buffer: WriteBuffer,
}

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Create a new bus using pre-allocated buffers.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	pub fn with_buffers(
		serial_port: impl Into<T>,
		read_buffer: ReadBuffer,
		write_buffer: WriteBuffer,
	) -> Result<Self, T::Error> {
		let serial_port = serial_port.into();
		let baud_rate = serial_port.baud_rate()?;
		Ok(Self::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate))
	}

	/// Create a new bus using pre-allocated buffers.
	pub fn with_buffers_and_baud_rate(
		serial_port: impl Into<T>,
		read_buffer: ReadBuffer,
		mut write_buffer: WriteBuffer,
		baud_rate: u32,
	) -> Self {
		// Pre-fill write buffer with the header prefix.
		// TODO: return Err instead of panicking.
		assert!(write_buffer.as_mut().len() >= INSTRUCTION_HEADER_SIZE + 2);
		write_buffer.as_mut()[..4].copy_from_slice(&HEADER_PREFIX);

		Self {
			serial_port: serial_port.into(),
			baud_rate,
			read_buffer,
			read_len: 0,
			used_bytes: 0,
			write_buffer,
		}
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), T::Error> {
		self.serial_port.set_baud_rate(baud_rate)?;
		self.baud_rate = baud_rate;
		Ok(())
	}

	pub fn write_status<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		error: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
	{
		crate::error::BufferTooSmallError::check(STATUS_HEADER_SIZE + parameter_count + 2, self.write_buffer.as_ref().len())?;
		self.write_instruction(packet_id, instruction_id, parameter_count + 1, |buffer| {
			buffer[0] = error;
			encode_parameters(&mut buffer[1..]);
		})
	}

	/// Write an instruction message to the bus.
	pub fn write_instruction<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<T::Error>>
	where
		F: FnOnce(&mut [u8]),
	{
		let buffer = self.write_buffer.as_mut();

		// Check if the buffer can hold the unstuffed message.
		crate::error::BufferTooSmallError::check(INSTRUCTION_HEADER_SIZE + parameter_count + 2, buffer.len())?;

		// Add the header, with a placeholder for the length field.
		buffer[4] = packet_id;
		buffer[5] = 0;
		buffer[6] = 0;
		buffer[7] = instruction_id;
		// The error byte for StatusPackets gets added in
		encode_parameters(&mut buffer[INSTRUCTION_HEADER_SIZE..][..parameter_count]);

		// Perform bitstuffing on the body.
		// The header never needs stuffing.
		let stuffed_body_len = bytestuff::stuff_inplace(&mut buffer[INSTRUCTION_HEADER_SIZE..], parameter_count)?;

		write_u16_le(&mut buffer[5..], stuffed_body_len as u16 + 3);

		// Add checksum.
		let checksum_index = INSTRUCTION_HEADER_SIZE + stuffed_body_len;
		let checksum = calculate_checksum(0, &buffer[..checksum_index]);
		write_u16_le(&mut buffer[checksum_index..], checksum);

		// Throw away old data in the read buffer and the kernel read buffer.
		// We don't do this when reading a reply, because we might receive multiple replies for one instruction,
		// and read() can potentially read more than one reply per syscall.
		self.read_len = 0;
		self.used_bytes = 0;
		self.serial_port.discard_input_buffer().map_err(WriteError::DiscardBuffer)?;

		// Send message.
		let stuffed_message = &buffer[..checksum_index + 2];
		trace!("sending instruction: {:02X?}", stuffed_message);
		self.serial_port.write_all(stuffed_message).map_err(WriteError::Write)?;
		Ok(())
	}

	/// Read a raw status response from the bus with the given deadline.
	pub fn read_packet_response_timeout<'a, P: Packet<'a>>(&'a mut self, timeout: Duration) -> Result<P, ReadError<T::Error>> {
		// Check that the read buffer is large enough to hold atleast a status packet header.
		crate::error::BufferTooSmallError::check(P::HEADER_SIZE, self.read_buffer.as_mut().len())?;

		let deadline = self.serial_port.make_deadline(timeout);

		let stuffed_message_len = loop {
			self.remove_garbage();

			// The call to remove_garbage() removes all leading bytes that don't match a status header.
			// So if there's enough bytes left, it's a status header.
			if self.read_len > P::HEADER_SIZE {
				let read_buffer = &self.read_buffer.as_mut()[..self.read_len];
				let body_len = read_buffer[5] as usize + read_buffer[6] as usize * 256;
				let body_len = body_len - P::HEADER_OVERLAP; // Length includes some bytes which are already included in P::PACKET_HEADER_SIZE.

				// Check if the read buffer is large enough for the entire message.
				// We don't have to remove the read bytes, because `write_instruction()` already clears the read buffer.
				crate::error::BufferTooSmallError::check(P::HEADER_SIZE + body_len, self.read_buffer.as_mut().len())?;

				if self.read_len >= P::HEADER_SIZE + body_len {
					trace!("P::HEADER_SIZE: {}, body_len: {}", P::HEADER_SIZE, body_len);
					break P::HEADER_SIZE + body_len;
				}
			}

			// Try to read more data into the buffer.
			let new_data = self.serial_port.read(&mut self.read_buffer.as_mut()[self.read_len..], &deadline)
				.map_err(ReadError::Io)?;
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

		// Mark the whole message as "used_bytes", so that the next call to `remove_garbage()` removes it.
		self.used_bytes += stuffed_message_len;

		// Remove byte-stuffing from the parameters.
		let parameter_count = bytestuff::unstuff_inplace(&mut buffer[P::HEADER_SIZE..parameters_end]);

		// Wrap the data in a `StatusPacket`.
		let response = P::new(&self.read_buffer.as_ref()[..P::HEADER_SIZE + parameter_count]);
		// TODO MOVE THESE BACK TO BUS
		// crate::InvalidInstruction::check(response.instruction_id(), crate::instructions::instruction_id::STATUS)?;
		// crate::MotorError::check(response.error())?;
		Ok(response)
	}
}

impl<ReadBuffer, WriteBuffer, T> Bus<ReadBuffer, WriteBuffer, T>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
	T: SerialPort,
{
	/// Remove leading garbage data from the read buffer.
	fn remove_garbage(&mut self) {
		let read_buffer = self.read_buffer.as_mut();
		let garbage_len = find_header(&read_buffer[..self.read_len][self.used_bytes..]);
		if garbage_len > 0 {
			debug!("skipping {} bytes of leading garbage.", garbage_len);
			trace!("skipped garbage: {:02X?}", &read_buffer[..garbage_len]);
		}
		self.consume_read_bytes(self.used_bytes + garbage_len);
		debug_assert_eq!(self.used_bytes, 0);
	}

	fn consume_read_bytes(&mut self, len: usize) {
		debug_assert!(len <= self.read_len);
		self.read_buffer.as_mut().copy_within(len..self.read_len, 0);
		// Decrease both used_bytes and read_len together.
		// Some consumed bytes may be garbage instead of used bytes though.
		// So we use `saturating_sub` for `used_bytes` to cap the result at 0.
		self.used_bytes = self.used_bytes.saturating_sub(len);
		self.read_len -= len;
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

/// Calculate the required time to transfer a message of a given size.
///
/// The size must include any headers and footers of the message.
pub(crate) fn message_transfer_time(message_size: u32, baud_rate: u32) -> Duration {
	let baud_rate = u64::from(baud_rate);
	let bits = u64::from(message_size) * 10; // each byte is 1 start bit, 8 data bits and 1 stop bit.
	let secs = bits / baud_rate;
	let subsec_bits = bits % baud_rate;
	let nanos = (subsec_bits * 1_000_000_000).div_ceil(baud_rate);
	Duration::new(secs, nanos as u32)
}

#[cfg(test)]
mod test {
	use super::*;
	use assert2::assert;

	#[test]
	fn test_message_transfer_time() {
		// Try a bunch of values to ensure we dealt with overflow correctly.
		assert!(message_transfer_time(100, 1_000) == Duration::from_secs(1));
		assert!(message_transfer_time(1_000, 10_000) == Duration::from_secs(1));
		assert!(message_transfer_time(1_000, 1_000_000) == Duration::from_millis(10));
		assert!(message_transfer_time(1_000, 10_000_000) == Duration::from_millis(1));
		assert!(message_transfer_time(1_000, 100_000_000) == Duration::from_micros(100));
		assert!(message_transfer_time(1_000, 1_000_000_000) == Duration::from_micros(10));
		assert!(message_transfer_time(1_000, 2_000_000_000) == Duration::from_micros(5));
		assert!(message_transfer_time(1_000, 4_000_000_000) == Duration::from_nanos(2500));
		assert!(message_transfer_time(10_000, 4_000_000_000) == Duration::from_micros(25));
		assert!(message_transfer_time(1_000_000, 4_000_000_000) == Duration::from_micros(2500));
		assert!(message_transfer_time(10_000_000, 4_000_000_000) == Duration::from_millis(25));
		assert!(message_transfer_time(100_000_000, 4_000_000_000) == Duration::from_millis(250));
		assert!(message_transfer_time(1_000_000_000, 4_000_000_000) == Duration::from_millis(2500));
		assert!(message_transfer_time(2_000_000_000, 4_000_000_000) == Duration::from_secs(5));
		assert!(message_transfer_time(4_000_000_000, 4_000_000_000) == Duration::from_secs(10));
		assert!(message_transfer_time(4_000_000_000, 2_000_000_000) == Duration::from_secs(20));
		assert!(message_transfer_time(4_000_000_000, 1_000_000_000) == Duration::from_secs(40));
		assert!(message_transfer_time(4_000_000_000, 100_000_000) == Duration::from_secs(400));
		assert!(message_transfer_time(4_000_000_000, 10_000_000) == Duration::from_secs(4_000));
		assert!(message_transfer_time(4_000_000_000, 1_000_000) == Duration::from_secs(40_000));
		assert!(message_transfer_time(4_000_000_000, 100_000) == Duration::from_secs(400_000));
		assert!(message_transfer_time(4_000_000_000, 10_000) == Duration::from_secs(4_000_000));
		assert!(message_transfer_time(4_000_000_000, 1_000) == Duration::from_secs(40_000_000));
		assert!(message_transfer_time(4_000_000_000, 100) == Duration::from_secs(400_000_000));
		assert!(message_transfer_time(4_000_000_000, 10) == Duration::from_secs(4_000_000_000));
		assert!(message_transfer_time(4_000_000_000, 1) == Duration::from_secs(40_000_000_000));

		assert!(message_transfer_time(43, 1) == Duration::from_secs(430));
		assert!(message_transfer_time(43, 10) == Duration::from_secs(43));
		assert!(message_transfer_time(43, 2) == Duration::from_secs(215));
		assert!(message_transfer_time(43, 20) == Duration::from_millis(21_500));
		assert!(message_transfer_time(43, 200) == Duration::from_millis(2_150));
		assert!(message_transfer_time(43, 2_000_000) == Duration::from_micros(215));
		assert!(message_transfer_time(43, 2_000_000_000) == Duration::from_nanos(215));
		assert!(message_transfer_time(43, 4_000_000_000) == Duration::from_nanos(108)); // rounded up
		assert!(message_transfer_time(3, 4_000_000_000) == Duration::from_nanos(8)); // rounded up
		assert!(message_transfer_time(5, 4_000_000_000) == Duration::from_nanos(13)); // rounded up

		let lots = u32::MAX - 1; // Use MAX - 1 because MAX is not cleanly divisible by 2.
		assert!(message_transfer_time(lots, 1) == Duration::from_secs(u64::from(lots) * 10));
		assert!(message_transfer_time(lots, lots) == Duration::from_secs(10));
		assert!(message_transfer_time(lots / 2, lots) == Duration::from_secs(5));
		assert!(message_transfer_time(lots, lots / 2) == Duration::from_secs(20));
	}

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
