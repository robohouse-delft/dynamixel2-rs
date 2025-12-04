//! Low level interface to a DYNAMIXEL Protocol 2.0 bus.

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
}
