//! Low level interface to a DYNAMIXEL Protocol 2.0 bus.

use crate::{checksum, ReadError, WriteError};
use core::time::Duration;

pub(crate) mod bytestuff;
pub(crate) mod endian;

pub(crate) mod data;
pub use data::Data;

mod packet;
pub use packet::{InstructionPacket, Packet, StatusPacket};

/// Prefix of a packet.
///
/// All packets start with this prefix, and they can not contain it in the body.
///
/// In other words: if you see this in the data stream, it must be the start of a packet.
const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];

/// The size of a message header, including the pre-amble, packet ID and length.
///
/// Excludes the instruction ID, the error field of status packets, the parameters and the CRC.
const HEADER_SIZE: usize = 7;

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

/// Create a mutable static buffer of size N.
///
/// This macro returns a `&'mut static [u8]`.
/// Each occurence of the macro may only be evaluated once,
/// any subsequent invocation will panic.
///
/// The macro is usable as expression in const context.
///
/// # Usage:
/// ```no_run
/// # fn main() -> Result<(), std::io::Error> {
/// # let serial_port = serial2::SerialPort::open("/dev/null", 57600)?;
/// use dynamixel2::{Client, static_buffer};
/// let client = Client::with_buffers(serial_port, static_buffer!(128), static_buffer!(64))?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! static_buffer {
	($N:literal) => {{
		use ::core::sync::atomic::{AtomicBool, Ordering};
		static USED: AtomicBool = AtomicBool::new(false);
		static mut BUFFER: [u8; $N] = [0; $N];
		if USED.swap(true, Ordering::Relaxed) {
			panic!("static buffer already used, each occurence of `static_buffer!()` may only be evaluated once");
		}
		unsafe {
			// Use raw pointer to avoid compiler warning.
			let buffer = &raw mut BUFFER;
			// Convert to reference in separate expression to avoid clippy warning.
			let buffer = &mut *buffer;
			buffer.as_mut_slice()
		}
	}};
}

/// Low level interface to a DYNAMIXEL Protocol 2.0 bus.
///
/// Does not assume anything about the direction of communication.
/// Used by [`crate::Client`] and [`crate::Device`].
pub(crate) struct Bus<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
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

impl<SerialPort, Buffer> Bus<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
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
	pub fn write_status<F>(
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
		self.write_packet(
			packet_id,
			crate::instructions::instruction_id::STATUS,
			parameter_count + 1,
			|buffer| {
				buffer[0] = error;
				encode_parameters(&mut buffer[1..])
			},
		)
	}

	/// Write an instruction message to the bus.
	pub fn write_instruction<F>(
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
	}

	/// Write a packet to the bus.
	pub fn write_packet<F>(
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
		self.serial_port.write_all(stuffed_message).map_err(WriteError::Write)?;
		Ok(())
	}

	/// Read a raw packet from the bus with the given deadline.
	pub fn read_packet_deadline(&mut self, deadline: SerialPort::Instant) -> Result<Packet<'_>, ReadError<SerialPort::Error>> {
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
		let packet = packet::Packet { data };

		// Ensure that status packets have an error field (included in parameter_count here).
		if packet.instruction_id() == crate::instructions::instruction_id::STATUS && parameter_count < 1 {
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

	#[test]
	fn test_static_buffer() {
		let buffer1 = static_buffer!(128);
		assert!(buffer1.len() == 128);
		for e in buffer1 {
			assert!(*e == 0);
		}

		let buffer2 = static_buffer!(64);
		assert!(buffer2.len() == 64);
		for e in buffer2 {
			assert!(*e == 0);
		}
	}

	#[test]
	#[should_panic]
	fn test_static_buffer_panics_when_evaluated_in_loop() {
		for _ in 0..2 {
			let buffer = static_buffer!(128);
			assert!(buffer.len() == 128);
		}
	}

	#[test]
	#[should_panic]
	fn test_static_buffer_panics_when_evaluated_in_loop_through_function() {
		fn make_buffer() -> &'static mut [u8] {
			static_buffer!(128)
		}

		for _ in 0..2 {
			let buffer = make_buffer();
			assert!(buffer.len() == 128);
		}
	}
}
