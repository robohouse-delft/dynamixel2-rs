//! Low level interface to a DYNAMIXEL Protocol 2.0 bus.

use crate::bus_types::{bytestuff, endian, InstructionPacket, Packet, StatusPacket, HEADER_PREFIX, HEADER_SIZE};
use crate::{checksum, ReadError, WriteError};

/// Default buffer type.
///
/// Defaults to [`Vec<u8>`] if the `"alloc"` or `"std"` feature is enabled.
/// Otherwise, defaults to `&'mut static [u8]`.
#[cfg(feature = "alloc")]
pub type DefaultBuffer = alloc::vec::Vec<u8>;

/// Default buffer type.
///
/// Defaults to [`Vec<u8>`] if the `"alloc"` or `"std"` feature is enabled.
/// Otherwise, defaults to `&'mut static [u8]`.
#[cfg(not(feature = "alloc"))]
pub type DefaultBuffer = &'static mut [u8];

/// Low level interface to a DYNAMIXEL Protocol 2.0 bus.
///
/// Does not assume anything about the direction of communication.
/// Used by [`crate::Client`] and [`crate::Device`].
pub(crate) struct Bus<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// The underlying stream (normally a serial port).
	pub(crate) serial_port: SerialPort,

	/// The baud rate of the serial port, if known.
	pub(crate) baud_rate: u32,

	/// The buffer for reading incoming messages.
	pub(crate) read_buffer: Buffer,

	/// The total number of valid bytes in the read buffer.
	pub(crate) read_len: usize,

	/// The number of leading bytes in the read buffer that have already been used.
	pub(crate) used_bytes: usize,

	/// The buffer for outgoing messages.
	pub(crate) write_buffer: Buffer,
}

#[super::bisync]
impl<SerialPort, Buffer> Bus<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Create a new bus using pre-allocated buffers.
	///
	/// The serial port must already be configured in raw mode with the correct baud rate,
	/// character size (8), parity (disabled) and stop bits (1).
	pub fn with_buffers(serial_port: SerialPort, read_buffer: Buffer, write_buffer: Buffer) -> Result<Self, SerialPort::Error> {
		let baud_rate = serial_port.baud_rate()?;
		Ok(Self::with_buffers_and_baud_rate(serial_port, read_buffer, write_buffer, baud_rate))
	}

	/// Create a new bus using pre-allocated buffers.
	pub fn with_buffers_and_baud_rate(serial_port: SerialPort, read_buffer: Buffer, write_buffer: Buffer, baud_rate: u32) -> Self {
		let mut write_buffer = write_buffer;

		// Pre-fill write buffer with the header prefix.
		// TODO: return Err instead of panicking.
		assert!(write_buffer.as_mut().len() >= HEADER_SIZE + 3);
		write_buffer.as_mut()[..4].copy_from_slice(&HEADER_PREFIX);

		Self {
			serial_port,
			baud_rate,
			read_buffer,
			read_len: 0,
			used_bytes: 0,
			write_buffer,
		}
	}

	/// Set the baud rate of the underlying serial port.
	pub fn set_baud_rate(&mut self, baud_rate: u32) -> Result<(), SerialPort::Error> {
		self.serial_port.set_baud_rate(baud_rate)?;
		self.baud_rate = baud_rate;
		Ok(())
	}

	/// Write a status message to the bus.
	pub async fn write_status<F>(
		&mut self,
		packet_id: u8,
		error: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<SerialPort::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		crate::error::BufferTooSmallError::check(StatusPacket::message_len(parameter_count), self.write_buffer.as_ref().len())?;
		self.write_packet(packet_id, crate::instruction_id::STATUS, parameter_count + 1, |buffer| {
			buffer[0] = error;
			encode_parameters(&mut buffer[1..])
		})
		.await
	}

	/// Write an instruction message to the bus.
	pub async fn write_instruction<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<SerialPort::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		self.write_packet(packet_id, instruction_id, parameter_count, encode_parameters)
			.await
	}

	/// Write a packet to the bus.
	pub async fn write_packet<F>(
		&mut self,
		packet_id: u8,
		instruction_id: u8,
		parameter_count: usize,
		encode_parameters: F,
	) -> Result<(), WriteError<SerialPort::Error>>
	where
		F: FnOnce(&mut [u8]) -> Result<(), crate::error::BufferTooSmallError>,
	{
		let buffer = self.write_buffer.as_mut();

		// Check if the buffer can hold the unstuffed message.
		crate::error::BufferTooSmallError::check(InstructionPacket::message_len(parameter_count), buffer.len())?;

		// Add the header, with a placeholder for the length field.
		buffer[4] = packet_id;
		buffer[5] = 0;
		buffer[6] = 0;
		buffer[7] = instruction_id;
		encode_parameters(&mut buffer[8..][..parameter_count])?;

		// Perform bitstuffing on the body.
		// The header never needs stuffing.
		// However, strictly following the spec, the instruction ID might need stuffing.
		let stuffed_body_len = bytestuff::stuff_inplace(&mut buffer[HEADER_SIZE..], 1 + parameter_count)?;

		endian::write_u16_le(&mut buffer[5..], stuffed_body_len as u16 + 2);

		// Add checksum.
		let checksum_index = HEADER_SIZE + stuffed_body_len;
		let checksum = checksum::calculate_checksum(0, &buffer[..checksum_index]);
		endian::write_u16_le(&mut buffer[checksum_index..], checksum);

		// Throw away old data in the read buffer and the kernel read buffer.
		// We don't do this when reading a reply, because we might receive multiple replies for one instruction,
		// and read() can potentially read more than one reply per syscall.
		self.read_len = 0;
		self.used_bytes = 0;
		self.serial_port.discard_input_buffer().map_err(WriteError::DiscardBuffer)?;

		// Send message.
		let stuffed_message = &buffer[..checksum_index + 2];
		trace!("sending packet: {:02X?}", stuffed_message);
		self.serial_port.write_all(stuffed_message).await.map_err(WriteError::Write)?;
		Ok(())
	}

	/// Read a raw packet from the bus with the given deadline.
	pub async fn read_packet_deadline(&mut self, deadline: SerialPort::Instant) -> Result<Packet<'_>, ReadError<SerialPort::Error>> {
		// Check that the read buffer is large enough to hold atleast a instruction packet with 0 parameters.
		crate::error::BufferTooSmallError::check(HEADER_SIZE + 3, self.read_buffer.as_mut().len())?;

		let stuffed_message_len = loop {
			self.remove_garbage();

			// The call to remove_garbage() removes all leading bytes that don't match a packet header.
			// So if there's enough bytes left, it's a packet header.
			if self.read_len > HEADER_SIZE {
				let read_buffer = &self.read_buffer.as_mut()[..self.read_len];
				let body_len = endian::read_u16_le(&read_buffer[5..]) as usize;

				// Check if the read buffer is large enough for the entire message.
				crate::error::BufferTooSmallError::check(HEADER_SIZE + body_len, self.read_buffer.as_mut().len()).inspect_err(|_| {
					self.consume_read_bytes(HEADER_SIZE);
				})?;

				if self.read_len >= HEADER_SIZE + body_len {
					break HEADER_SIZE + body_len;
				}
			}

			// Try to read more data into the buffer.
			let new_data = self
				.serial_port
				.read(&mut self.read_buffer.as_mut()[self.read_len..], &deadline)
				.await
				.map_err(ReadError::Io)?;

			self.read_len += new_data;
		};

		let buffer = self.read_buffer.as_mut();
		let parameters_end = stuffed_message_len - 2;
		trace!("read packet: {:02X?}", &buffer[..parameters_end]);

		let checksum_message = endian::read_u16_le(&buffer[parameters_end..]);
		let checksum_computed = checksum::calculate_checksum(0, &buffer[..parameters_end]);
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

		// Remove byte-stuffing from the everything from instruction ID to the parameters.
		let parameter_count = bytestuff::unstuff_inplace(&mut buffer[HEADER_SIZE..parameters_end]);

		// Wrap the data in a `Packet`.
		let data = &self.read_buffer.as_ref()[..HEADER_SIZE + parameter_count];
		let packet = Packet { data };

		// Ensure that status packets have an error field (included in parameter_count here).
		if packet.instruction_id() == crate::instruction_id::STATUS && parameter_count < 1 {
			return Err(crate::InvalidMessage::InvalidParameterCount(crate::InvalidParameterCount {
				actual: 0,
				expected: crate::ExpectedCount::Min(1),
			})
			.into());
		}

		Ok(packet)
	}

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

#[cfg(test)]
mod test {
	use core::time::Duration;

	use super::*;
	use crate::static_buffer;
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

	#[super::super::only_sync]
	#[test]
	fn test_buffer_too_small() {
		let read_buffer = crate::static_buffer!(128);
		let write_buffer = crate::static_buffer!(128);

		// Dummy serial port used to feed packages
		// It does not support writing them etc.
		struct DummySerial {}

		impl crate::SerialPort for DummySerial {
			type Error = std::io::Error;
			type Instant = std::time::Instant;

			fn baud_rate(&self) -> Result<u32, Self::Error> {
				Ok(115_200)
			}

			fn set_baud_rate(&mut self, _baud_rate: u32) -> Result<(), Self::Error> {
				unimplemented!("not used in this test")
			}

			fn discard_input_buffer(&mut self) -> Result<(), Self::Error> {
				unimplemented!("not used in this test")
			}

			fn read(&mut self, _buffer: &mut [u8], _deadline: &Self::Instant) -> Result<usize, Self::Error> {
				// Packet 1
				// Build a package header with a (wrong) length of 10000 in the header
				_buffer[..4].copy_from_slice(&HEADER_PREFIX);
				_buffer[4] = 0;
				endian::write_u16_le(&mut _buffer[5..], 10000);

				// Packet 2
				// Build a normal packet
				let offset = 8;
				let packet_2_length = 10;
				_buffer[offset..offset + 4].copy_from_slice(&HEADER_PREFIX);
				_buffer[offset + 4] = 0;
				endian::write_u16_le(&mut _buffer[offset + 5..], packet_2_length as u16);
				let checksum_len = 2;
				let checksum_index = HEADER_SIZE + packet_2_length - checksum_len;
				let checksum = checksum::calculate_checksum(0, &_buffer[offset..offset + checksum_index]);
				endian::write_u16_le(&mut _buffer[offset + checksum_index..], checksum);
				Ok(50)
			}

			fn write_all(&mut self, _buffer: &[u8]) -> Result<(), Self::Error> {
				unimplemented!("not used in this test")
			}

			fn make_deadline(&self, _timeout: core::time::Duration) -> Self::Instant {
				std::time::Instant::now() + _timeout
			}

			fn is_timeout_error(_error: &Self::Error) -> bool {
				unimplemented!("not used in this test")
			}
		}

		// Setup the bus with a dummy serial interface
		let mut bus = Bus::with_buffers(DummySerial {}, read_buffer, write_buffer).unwrap();

		// Read the corrupt package
		let deadline = std::time::Instant::now() + Duration::from_secs(1);
		let result = bus.read_packet_deadline(deadline);
		assert!(matches!(result.unwrap_err(), crate::ReadError::BufferFull(_)));

		// Check that the next read works normally again (buffer is partially flushed)
		let result = bus.read_packet_deadline(deadline);
		assert!(result.is_ok());
	}
}
