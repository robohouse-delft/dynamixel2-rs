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
	///
	/// A status packet can hold at most a `u16` worth of parameters.
	/// This function also panics if the combined response would exceed that
	/// (`sum(count + 4)` over all `reads`), which requires a pathological number of motors and registers.
	pub async fn fast_bulk_read_bytes<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<FastBulkRead<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		self.fast_bulk_read_raw(reads).await
	}

	/// Read arbitrary data ranges from multiple motors using the fast bulk read instruction, borrowing each
	/// reply from the read buffer.
	///
	/// This is like [`Self::fast_bulk_read_bytes`], except that each reply borrows its data directly from
	/// the internal read buffer instead of allocating. Consume the replies with
	/// [`FastBulkRead::read_next_borrow`], which yields a [`Response`] of `&T` (for example `&[u8]`).
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	///
	/// A status packet can hold at most a `u16` worth of parameters.
	/// This function also panics if the combined response would exceed that
	/// (`sum(count + 4)` over all `reads`), which requires a pathological number of motors and registers.
	pub async fn fast_bulk_read_bytes_borrow<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<FastBulkRead<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>>
	where
		T: ?Sized,
		[u8]: core::borrow::Borrow<T>,
	{
		self.fast_bulk_read_raw(reads).await
	}

	/// Send a fast bulk read instruction and read the combined status packet into a [`FastBulkRead`].
	///
	/// The way the replies are decoded (owned or borrowed) is decided by the caller through `T`.
	async fn fast_bulk_read_raw<'a, T: ?Sized>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<FastBulkRead<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>> {
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
		// A status packet can never carry more than a `u16` worth of parameters. Exceeding that needs a
		// pathological number of motors and registers (see the `# Panics` note), so treat it as a caller bug.
		let expected_parameters = reads.iter().fold(0u32, |acc, read| acc + u32::from(read.count) + 4);
		let expected_parameters = u16::try_from(expected_parameters)
			.expect("fast_bulk_read: the requested response is larger than a single status packet can hold");
		let response = self.read_status_response(expected_parameters, false).await?;
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
pub struct FastBulkRead<'a, T: ?Sized, E> {
	/// The unparsed per-motor blocks, starting at the error byte of the first motor.
	parameters: &'a [u8],

	/// The requested reads, used to know the data length of each motor block.
	reads: &'a [BulkReadData],

	/// The index of the next read to yield.
	index: usize,

	data: PhantomData<fn(&T) -> E>,
}

impl<T: ?Sized, E> core::fmt::Debug for FastBulkRead<'_, T, E> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FastBulkRead")
			.field("reads", &self.reads)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<'a, T: ?Sized, E> FastBulkRead<'a, T, E> {
	/// The number of motor replies that have not been yielded yet.
	pub fn remaining(&self) -> usize {
		self.reads.len() - self.index
	}

	/// Split the next `error + motor ID + data` block off the front of the buffer, advancing the state.
	fn next_block(&mut self) -> Option<Result<&'a [u8], ReadError<E>>> {
		let count = usize::from(self.reads.get(self.index)?.count);
		self.index += 1;

		// Each motor block is: error (1) + motor ID (1) + data (`count`).
		let block_len = 2 + count;
		let Some((block, rest)) = self.parameters.split_at_checked(block_len) else {
			// The response is shorter than `reads` implies: a motor block is missing or truncated.
			// Surface an error instead of silently ending iteration with motors unaccounted for.
			self.index = self.reads.len();
			return Some(Err(crate::InvalidParameterCount {
				actual: self.parameters.len(),
				expected: crate::ExpectedCount::Min(block_len),
			}
			.into()));
		};

		// Skip the per-motor CRC (2 bytes). The final motor's CRC doubles as the packet CRC and is
		// stripped while reading the packet, so it may be absent for the last block.
		self.parameters = rest.get(2..).unwrap_or(&[]);

		Some(Ok(block))
	}

	/// Read the next motor reply, or [`None`] once every read has been yielded.
	pub fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<E>>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		Some(match self.next_block()? {
			Ok(block) => decode_motor_block(block, |data| T::from(data)),
			Err(e) => Err(e),
		})
	}

	/// Read the next motor reply borrowing the data from the read buffer, or [`None`] once every read has been yielded.
	pub fn read_next_borrow(&mut self) -> Option<Result<Response<&'a T>, ReadError<E>>>
	where
		[u8]: core::borrow::Borrow<T>,
	{
		Some(match self.next_block()? {
			Ok(block) => decode_motor_block(block, core::borrow::Borrow::borrow),
			Err(e) => Err(e),
		})
	}
}

impl<T, E> Iterator for FastBulkRead<'_, T, E>
where
	T: for<'b> From<&'b [u8]>,
{
	type Item = Result<Response<T>, ReadError<E>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

/// Build a [`Response`] from a single `error + motor ID + data` block of a fast bulk read response.
///
/// `decode` turns the data bytes into the reply value, allowing either an owned (`T: From<&[u8]>`) or a
/// borrowed (`&T`) result to share the block-parsing logic.
fn decode_motor_block<'b, D, R, E>(block: &'b [u8], decode: D) -> Result<Response<R>, ReadError<E>>
where
	D: FnOnce(&'b [u8]) -> R,
{
	let error = block[0];
	MotorError::check(error)?;
	Ok(Response {
		motor_id: block[1],
		alert: error & 0x80 != 0,
		data: decode(&block[2..]),
	})
}

#[cfg(test)]
#[super::only_sync]
mod test {
	use super::FastBulkRead;
	use crate::client::BulkReadData;
	use crate::{InvalidMessage, ReadError, Response};
	use alloc::vec;
	use alloc::vec::Vec;
	use assert2::{assert, let_assert};
	use core::convert::Infallible;
	use core::marker::PhantomData;

	#[test]
	fn read_next_borrow_yields_slices() {
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
		let parameters = [
			0x00, 0x01, 0xAA, 0xBB, 0x11, 0x22, // motor 1: 2 data bytes [0xAA, 0xBB] + skipped CRC.
			0x00, 0x02, 0xCC, 0xDD, 0xEE, // motor 2: 3 data bytes [0xCC, 0xDD, 0xEE], CRC stripped.
		];
		let mut iter = FastBulkRead::<[u8], Infallible> {
			parameters: &parameters,
			reads: &reads,
			index: 0,
			data: PhantomData,
		};

		let_assert!(Some(Ok(response)) = iter.read_next_borrow());
		assert!(response.motor_id == 1);
		assert!(response.data == [0xAA, 0xBB].as_slice());

		let_assert!(Some(Ok(response)) = iter.read_next_borrow());
		assert!(response.motor_id == 2);
		assert!(response.data == [0xCC, 0xDD, 0xEE].as_slice());

		assert!(let None = iter.read_next_borrow());
	}

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

	#[test]
	fn errors_on_truncated_response() {
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
		// Two motors expected, but the buffer only holds the first block: the second motor is missing.
		let parameters = [0x00, 0x01, 0xAA, 0xBB];
		let mut iter = FastBulkRead::<Vec<u8>, Infallible> {
			parameters: &parameters,
			reads: &reads,
			index: 0,
			data: PhantomData,
		};

		let_assert!(Some(Ok(_)) = iter.next());
		// The missing motor surfaces as an error rather than a silent end of iteration.
		let_assert!(Some(Err(ReadError::InvalidMessage(InvalidMessage::InvalidParameterCount(_)))) = iter.next());
		assert!(let None = iter.next());
	}
}
