//! The fast sync read instruction, used to read the same data from multiple motors with a single status packet.
#![allow(unused_imports)]

use core::marker::PhantomData;

use super::Client;
use crate::bus::data::Data;
use crate::bus::endian::write_u16_le;
use crate::bus::{instruction_id, packet_id};
use crate::{MotorError, ReadError, Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read a value from multiple motors in one command using the fast sync read instruction.
	///
	/// Like [`crate::client::Client::sync_read`], the fast sync read instruction reads the same number of bytes from the same
	/// address from multiple motors. Unlike the regular sync read, all motors reply with a single status packet.
	/// This reduces the communication overhead, at the cost of losing the entire response if a single motor
	/// fails to reply.
	///
	/// The returned [`FastSyncRead`] is an iterator that yields the [`Response`] of each motor,
	/// in the same order as `motor_ids`.
	///
	/// # Panics
	/// A status packet can hold at most a `u16` worth of parameters.
	/// This function panics if the combined response would exceed that
	/// (`(T::ENCODED_SIZE + 4) * motor_ids.len()`), which requires a pathological number of motors and registers.
	pub async fn fast_sync_read<'a, T: Data>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
	) -> Result<FastSyncRead<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>> {
		let count = T::ENCODED_SIZE;
		self.write_instruction(
			packet_id::BROADCAST,
			instruction_id::FAST_SYNC_READ,
			4 + motor_ids.len(),
			|buffer| {
				write_u16_le(&mut buffer[0..], address);
				write_u16_le(&mut buffer[2..], count);
				buffer[4..].copy_from_slice(motor_ids);
				Ok(())
			},
		)
		.await?;

		// Each motor block in the response is: error (1) + motor ID (1) + data (`count`) + CRC (2).
		// A status packet can never carry more than a `u16` worth of parameters. Exceeding that needs a
		// pathological number of motors and registers (see the `# Panics` note), so treat it as a caller bug.
		let expected_parameters = (u32::from(count) + 4) * motor_ids.len() as u32;
		let expected_parameters = u16::try_from(expected_parameters)
			.expect("fast_sync_read: the requested response is larger than a single status packet can hold");
		let response = self.read_status_response(expected_parameters, false).await?;
		crate::InvalidPacketId::check(response.packet_id(), packet_id::BROADCAST)?;

		Ok(FastSyncRead {
			parameters: response.error_and_parameters(),
			count,
			remaining: motor_ids.len(),
			data: PhantomData,
		})
	}
}

/// A fast sync read operation that yields the parsed value of each motor.
///
/// Returned by [`Client::fast_sync_read`].
/// The entire response is read from the bus before this iterator is returned;
/// iterating it simply splits the response into the per-motor replies.
pub struct FastSyncRead<'a, T, E> {
	/// The unparsed per-motor blocks, starting at the error byte of the first motor.
	parameters: &'a [u8],

	/// The number of data bytes in each motor block.
	count: u16,

	/// The number of motor replies that have not been yielded yet.
	remaining: usize,

	data: PhantomData<fn() -> (T, E)>,
}

impl<T, E> core::fmt::Debug for FastSyncRead<'_, T, E> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FastSyncRead")
			.field("count", &self.count)
			.field("remaining", &self.remaining)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T: Data, E> Iterator for FastSyncRead<'_, T, E> {
	type Item = Result<Response<T>, ReadError<E>>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.remaining == 0 {
			return None;
		}

		// Split off one motor block: error (1) + motor ID (1) + data (`count`).
		let block_len = 2 + usize::from(self.count);
		let Some((block, rest)) = self.parameters.split_at_checked(block_len) else {
			// The response is shorter than `motor_ids` implies: a motor block is missing or truncated.
			// Surface an error instead of silently ending iteration with motors unaccounted for.
			self.remaining = 0;
			return Some(Err(crate::InvalidParameterCount {
				actual: self.parameters.len(),
				expected: crate::ExpectedCount::Min(block_len),
			}
			.into()));
		};

		// Skip the per-motor CRC (2 bytes). The final motor's CRC doubles as the packet CRC and is
		// stripped while reading the packet, so it may be absent for the last block.
		self.parameters = rest.get(2..).unwrap_or(&[]);
		self.remaining -= 1;

		Some(parse_motor_block(block))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining))
	}
}

/// Parse a single `error + motor ID + data` block from a fast sync read response.
fn parse_motor_block<T: Data, E>(block: &[u8]) -> Result<Response<T>, ReadError<E>> {
	let error = block[0];
	MotorError::check(error)?;
	Ok(Response {
		motor_id: block[1],
		alert: error & 0x80 != 0,
		data: T::decode(&block[2..])?,
	})
}

#[cfg(test)]
#[super::only_sync]
mod test {
	use super::FastSyncRead;
	use crate::{InvalidMessage, ReadError, Response};
	use assert2::{assert, let_assert};
	use core::convert::Infallible;
	use core::marker::PhantomData;

	/// Build a `FastSyncRead` directly from a raw parameter buffer (the region starting at the first error byte).
	fn fast_sync_read<T>(parameters: &[u8], count: u16, motors: usize) -> FastSyncRead<'_, T, Infallible> {
		FastSyncRead {
			parameters,
			count,
			remaining: motors,
			data: PhantomData,
		}
	}

	#[test]
	fn parses_each_motor_block() {
		// Two motors, 2 bytes each: [error, id, data_le, crc, crc] per block.
		// The final block's CRC is the packet CRC, which is stripped before parsing, so it is absent here.
		let parameters = [
			0x00, 0x01, 0x34, 0x12, 0xAA, 0xBB, // motor 1: 0x1234, with a (skipped) CRC.
			0x00, 0x02, 0x78, 0x56, // motor 2: 0x5678, CRC stripped as the packet CRC.
		];
		let mut iter = fast_sync_read::<u16>(&parameters, 2, 2);

		let_assert!(Some(Ok(response)) = iter.next());
		assert!(
			response
				== Response {
					motor_id: 1,
					alert: false,
					data: 0x1234
				}
		);

		let_assert!(Some(Ok(response)) = iter.next());
		assert!(
			response
				== Response {
					motor_id: 2,
					alert: false,
					data: 0x5678
				}
		);

		assert!(let None = iter.next());
	}

	#[test]
	fn reports_motor_error_and_alert() {
		// Motor 1 reports a hardware error with the alert bit set; motor 2 is fine.
		let parameters = [
			0x81, 0x01, 0x00, 0x00, 0xAA, 0xBB, // error 0x01 + alert bit (0x80).
			0x00, 0x02, 0x07, 0x00,
		];
		let mut iter = fast_sync_read::<u16>(&parameters, 2, 2);

		let_assert!(Some(Err(ReadError::MotorError(error))) = iter.next());
		assert!(error.error_number() == 0x01);

		// Iteration continues with the next motor despite the earlier motor error.
		let_assert!(Some(Ok(response)) = iter.next());
		assert!(
			response
				== Response {
					motor_id: 2,
					alert: false,
					data: 0x0007
				}
		);
	}

	#[test]
	fn errors_on_truncated_response() {
		// Two motors expected, but the buffer only holds one complete block: the second motor is missing.
		let parameters = [0x00, 0x01, 0x34, 0x12];
		let mut iter = fast_sync_read::<u16>(&parameters, 2, 2);

		let_assert!(Some(Ok(_)) = iter.next());
		// The missing motor surfaces as an error rather than a silent end of iteration.
		let_assert!(Some(Err(ReadError::InvalidMessage(InvalidMessage::InvalidParameterCount(_)))) = iter.next());
		assert!(let None = iter.next());
	}
}
