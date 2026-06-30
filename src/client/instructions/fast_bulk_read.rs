//! The fast bulk read instruction, used to read different data from multiple motors with a single status packet.
#![allow(unused_imports)]

use core::marker::PhantomData;

use super::Client;
use crate::bus::endian::{write_u16_le, write_u8_le};
use crate::bus::{instruction_id, packet_id};
use crate::client::BulkReadData;
use crate::{MotorError, ReadError, Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read arbitrary data ranges from multiple motors using the fast bulk read instruction.
	///
	/// Like [`crate::client::Client::bulk_read_bytes`], a bulk read can read a different amount of data from a different address
	/// for each motor. Unlike the regular bulk read, all motors reply with a single status packet.
	/// This reduces the communication overhead, at the cost of losing the entire response if a single motor
	/// fails to reply.
	///
	/// The returned [`FastBulkRead`] is an iterator that yields the [`Response`] of each motor,
	/// in the same order as `reads`. The data of each response is returned as unparsed bytes.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub async fn fast_bulk_read_bytes<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<FastBulkRead<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		for i in 0..reads.len() {
			for j in i + 1..reads.len() {
				if reads[i].motor_id == reads[j].motor_id {
					panic!(
						"fast_bulk_read: motor ID {} used multiple times at index {} and {}",
						reads[i].motor_id, i, j
					)
				}
			}
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::FAST_BULK_READ, 5 * reads.len(), |buffer| {
			for (i, read) in reads.iter().enumerate() {
				let buffer = &mut buffer[i * 5..][..5];
				write_u8_le(&mut buffer[0..], read.motor_id);
				write_u16_le(&mut buffer[1..], read.address);
				write_u16_le(&mut buffer[3..], read.count);
			}
			Ok(())
		})
		.await?;

		// Each motor block in the response is: error (1) + motor ID (1) + data (`count`) + CRC (2).
		let expected_parameters = reads.iter().fold(0u32, |acc, read| acc + u32::from(read.count) + 4);
		let response = self.read_status_response(expected_parameters as u16, false).await?;
		crate::InvalidPacketId::check(response.packet_id(), packet_id::BROADCAST)?;

		Ok(FastBulkRead {
			parameters: response.error_and_parameters(),
			reads,
			index: 0,
			data: PhantomData,
		})
	}
}

/// A fast bulk read operation that yields the unparsed bytes read from each motor.
///
/// Returned by [`Client::fast_bulk_read_bytes`].
/// The entire response is read from the bus before this iterator is returned;
/// iterating it simply splits the response into the per-motor replies.
pub struct FastBulkRead<'a, T, E> {
	/// The unparsed per-motor blocks, starting at the error byte of the first motor.
	parameters: &'a [u8],

	/// The requested reads, used to know the data length of each motor block.
	reads: &'a [BulkReadData],

	/// The index of the next read to yield.
	index: usize,

	data: PhantomData<fn() -> (T, E)>,
}

impl<T, E> core::fmt::Debug for FastBulkRead<'_, T, E> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FastBulkRead")
			.field("reads", &self.reads)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T, E> Iterator for FastBulkRead<'_, T, E>
where
	T: for<'b> From<&'b [u8]>,
{
	type Item = Result<Response<T>, ReadError<E>>;

	fn next(&mut self) -> Option<Self::Item> {
		let count = usize::from(self.reads.get(self.index)?.count);
		self.index += 1;

		// Split off one motor block: error (1) + motor ID (1) + data (`count`).
		let (block, rest) = self.parameters.split_at_checked(2 + count)?;

		// Skip the per-motor CRC (2 bytes). The final motor's CRC doubles as the packet CRC and is
		// stripped while reading the packet, so it may be absent for the last block.
		self.parameters = rest.get(2..).unwrap_or(&[]);

		let error = block[0];
		Some(parse_motor_block(error, block[1], &block[2..]))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.reads.len() - self.index;
		(0, Some(remaining))
	}
}

/// Build a [`Response`] from a single `error + motor ID + data` block of a fast bulk read response.
fn parse_motor_block<T, E>(error: u8, motor_id: u8, data: &[u8]) -> Result<Response<T>, ReadError<E>>
where
	T: for<'b> From<&'b [u8]>,
{
	MotorError::check(error)?;
	Ok(Response {
		motor_id,
		alert: error & 0x80 != 0,
		data: T::from(data),
	})
}

#[cfg(test)]
#[super::only_sync]
mod test {
	use super::FastBulkRead;
	use crate::client::BulkReadData;
	use crate::Response;
	use alloc::vec;
	use alloc::vec::Vec;
	use assert2::{assert, let_assert};
	use core::convert::Infallible;
	use core::marker::PhantomData;

	#[test]
	fn parses_variable_length_blocks() {
		let reads = [
			BulkReadData {
				motor_id: 1,
				address: 0,
				count: 2,
			},
			BulkReadData {
				motor_id: 2,
				address: 0,
				count: 3,
			},
		];
		// Per-motor block: [error, id, data..., crc, crc]. The final block's CRC is the stripped packet CRC.
		let parameters = [
			0x00, 0x01, 0xAA, 0xBB, 0x11, 0x22, // motor 1: 2 data bytes [0xAA, 0xBB] + skipped CRC.
			0x00, 0x02, 0xCC, 0xDD, 0xEE, // motor 2: 3 data bytes [0xCC, 0xDD, 0xEE], CRC stripped.
		];
		let mut iter = FastBulkRead::<Vec<u8>, Infallible> {
			parameters: &parameters,
			reads: &reads,
			index: 0,
			data: PhantomData,
		};

		let_assert!(Some(Ok(response)) = iter.next());
		assert!(
			response
				== Response {
					motor_id: 1,
					alert: false,
					data: vec![0xAA, 0xBB]
				}
		);

		let_assert!(Some(Ok(response)) = iter.next());
		assert!(
			response
				== Response {
					motor_id: 2,
					alert: false,
					data: vec![0xCC, 0xDD, 0xEE]
				}
		);

		assert!(let None = iter.next());
	}
}
