//! Low level interface to a DYNAMIXEL Protocol 2.0 bus.

#[path = "."]
pub(crate) mod asynch {
	use crate::AsyncSerialPort as SerialPort;
	use bisync::asynchronous::*;
	mod bus;
	pub(crate) use bus::Bus;
}
#[path = "."]
pub(crate) mod sync {
	use crate::SerialPort;
	use bisync::synchronous::*;
	mod bus;
	pub(crate) use bus::Bus;
}

// pub(crate) use asynch::bus::Bus as AsyncBus;
// pub(crate) use sync::bus::Bus;

use core::time::Duration;

pub(crate) mod bytestuff;
pub(crate) mod endian;

pub(crate) mod data;
pub use data::Data;

mod packet;
pub use packet::{InstructionPacket, Packet, StatusPacket};

/// Raw instructions IDs.
#[rustfmt::skip]
#[allow(missing_docs)]
pub mod instruction_id {
	pub const PING          : u8 = 0x01;
	pub const READ          : u8 = 0x02;
	pub const WRITE         : u8 = 0x03;
	pub const REG_WRITE     : u8 = 0x04;
	pub const ACTION        : u8 = 0x05;
	pub const FACTORY_RESET : u8 = 0x06;
	pub const REBOOT        : u8 = 0x08;
	pub const CLEAR         : u8 = 0x10;
	pub const SYNC_READ     : u8 = 0x82;
	pub const SYNC_WRITE    : u8 = 0x83;
	pub const BULK_READ     : u8 = 0x92;
	pub const BULK_WRITE    : u8 = 0x93;
	pub const STATUS        : u8 = 0x55;
}

/// Special packet IDs.
pub mod packet_id {
	/// The broadcast address.
	pub const BROADCAST: u8 = 0xFE;
}

/// Prefix of a packet.
///
/// All packets start with this prefix, and they can not contain it in the body.
///
/// In other words: if you see this in the data stream, it must be the start of a packet.
pub(crate) const HEADER_PREFIX: [u8; 4] = [0xFF, 0xFF, 0xFD, 0x00];

/// The size of a message header, including the pre-amble, packet ID and length.
///
/// Excludes the instruction ID, the error field of status packets, the parameters and the CRC.
pub(crate) const HEADER_SIZE: usize = 7;

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
	#[allow(clippy::needless_range_loop, reason = "indexing gives better error messsage on failed assertion")]
	fn test_static_buffer() {
		let buffer1 = static_buffer!(128);
		assert!(buffer1.len() == 128);
		for i in 0..buffer1.len() {
			assert!(buffer1[i] == 0, "i: {i}");
		}

		let buffer2 = static_buffer!(64);
		assert!(buffer2.len() == 64);
		for i in 0..buffer2.len() {
			assert!(buffer2[i] == 0, "i: {i}");
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
	use core::time::Duration;

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
				let checksum = crate::checksum::calculate_checksum(0, &_buffer[offset..offset + checksum_index]);
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
		let mut bus = sync::Bus::with_buffers(DummySerial {}, read_buffer, write_buffer).unwrap();

		// Read the corrupt package
		let deadline = std::time::Instant::now() + Duration::from_secs(1);
		let result = bus.read_packet_deadline(deadline);
		assert!(matches!(result.unwrap_err(), crate::ReadError::BufferFull(_)));

		// Check that the next read works normally again (buffer is partially flushed)
		let result = bus.read_packet_deadline(deadline);
		assert!(result.is_ok());
	}
}
