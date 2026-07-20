//! The fast sync read instruction, used to read the same data from multiple motors with a single status packet.
#![allow(unused_imports)]

use core::marker::PhantomData;

use super::Client;
use crate::bus::data::Data;
use crate::bus::endian::write_u16_le;
use crate::bus::{instruction_id, packet_id};
use crate::{MissingResponse, MotorError, ReadError, Response, TransferError};

#[super::bisync]
impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: super::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Read a value from multiple motors in one command using the fast sync read instruction.
	///
	/// Like [`Self::sync_read`], the fast sync read instruction reads the same number of bytes from the same
	/// address from multiple motors. Unlike the regular sync read, all motors reply with a single status packet.
	/// This reduces the communication overhead.
	///
	/// The returned [`FastSyncRead`] is an iterator that yields the [`Response`] of each motor,
	/// in the same order as `motor_ids`.
	///
	/// If a motor fails to reply, the replies of the motors before it are still yielded, and the iterator
	/// then yields a [`MissingResponse`] error naming the first motor that did not respond.
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
		let motor_count = motor_ids.len();
		let response = self
			.read_fast_read_response(expected_parameters, |i| (i < motor_count).then_some(usize::from(count)))
			.await?;
		crate::InvalidPacketId::check(response.packet_id(), packet_id::BROADCAST)?;

		Ok(FastSyncRead {
			parameters: response.error_and_parameters(),
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		})
	}

	/// Read a number of bytes from multiple motors in one command using the fast sync read instruction.
	///
	/// This is like [`Self::fast_sync_read`], except that the number of bytes to read is given at runtime
	/// as `count` and each reply is returned as unparsed bytes rather than a parsed [`Data`] value.
	///
	/// The returned [`FastSyncReadBytes`] is an iterator that yields the [`Response`] of each motor,
	/// in the same order as `motor_ids`.
	///
	/// # Panics
	/// A status packet can hold at most a `u16` worth of parameters.
	/// This function panics if the combined response would exceed that
	/// (`(count + 4) * motor_ids.len()`), which requires a pathological number of motors and registers.
	pub async fn fast_sync_read_bytes<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<FastSyncReadBytes<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		self.fast_sync_read_raw(motor_ids, address, count).await
	}

	/// Read a number of bytes from multiple motors in one command using the fast sync read instruction,
	/// borrowing each reply from the read buffer.
	///
	/// This is like [`Self::fast_sync_read_bytes`], except that each reply borrows its data directly from
	/// the internal read buffer instead of allocating. Consume the replies with
	/// [`FastSyncReadBytes::read_next_borrow`], which yields a [`Response`] of `&T` (for example `&[u8]`).
	///
	/// # Panics
	/// A status packet can hold at most a `u16` worth of parameters.
	/// This function panics if the combined response would exceed that
	/// (`(count + 4) * motor_ids.len()`), which requires a pathological number of motors and registers.
	pub async fn fast_sync_read_bytes_borrow<'a, T>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<FastSyncReadBytes<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>>
	where
		T: ?Sized,
		[u8]: core::borrow::Borrow<T>,
	{
		self.fast_sync_read_raw(motor_ids, address, count).await
	}

	/// Send a fast sync read instruction and read the combined status packet into a [`FastSyncReadBytes`].
	///
	/// The way the replies are decoded (owned or borrowed) is decided by the caller through `T`.
	async fn fast_sync_read_raw<'a, T: ?Sized>(
		&'a mut self,
		motor_ids: &'a [u8],
		address: u16,
		count: u16,
	) -> Result<FastSyncReadBytes<'a, T, SerialPort::Error>, TransferError<SerialPort::Error>> {
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
		let motor_count = motor_ids.len();
		let response = self
			.read_fast_read_response(expected_parameters, |i| (i < motor_count).then_some(usize::from(count)))
			.await?;
		crate::InvalidPacketId::check(response.packet_id(), packet_id::BROADCAST)?;

		Ok(FastSyncReadBytes {
			parameters: response.error_and_parameters(),
			count,
			motor_ids,
			index: 0,
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

	/// The IDs of the motors that were addressed, in reply order.
	motor_ids: &'a [u8],

	/// The index of the next motor reply to yield.
	index: usize,

	data: PhantomData<fn() -> (T, E)>,
}

impl<T, E> core::fmt::Debug for FastSyncRead<'_, T, E> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FastSyncRead")
			.field("count", &self.count)
			.field("motor_ids", &self.motor_ids)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T, E> FastSyncRead<'_, T, E> {
	/// The number of motor replies that have not been yielded yet.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}
}

impl<T: Data, E> Iterator for FastSyncRead<'_, T, E> {
	type Item = Result<Response<T>, ReadError<E>>;

	fn next(&mut self) -> Option<Self::Item> {
		let block = match split_motor_block(&mut self.parameters, self.motor_ids, &mut self.index, usize::from(self.count))? {
			Ok(block) => block,
			Err(e) => return Some(Err(e)),
		};
		Some(parse_motor_block(block))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

/// Split the next `error + motor ID + data` block off the front of a fast sync read response.
///
/// Advances `index` and trims `parameters`. Returns [`None`] once every motor has been accounted for. If the
/// expected block is missing (the response ran out early) or the block belongs to a different motor than
/// expected, the culprit is reported as a [`MissingResponse`] and iteration is ended.
fn split_motor_block<'a, E>(
	parameters: &mut &'a [u8],
	motor_ids: &[u8],
	index: &mut usize,
	count: usize,
) -> Option<Result<&'a [u8], ReadError<E>>> {
	let motor_id = *motor_ids.get(*index)?;
	*index += 1;

	// Split off one motor block: error (1) + motor ID (1) + data (`count`).
	let block_len = 2 + count;
	let Some((block, rest)) = parameters.split_at_checked(block_len) else {
		// The response is shorter than `motor_ids` implies: this motor's block never arrived.
		*index = motor_ids.len();
		return Some(Err(MissingResponse { motor_id }.into()));
	};

	// A block is present but belongs to a different motor: an earlier motor's reply is missing and the
	// stream has desynced. Report the expected motor as missing rather than mislabelling later data.
	if block[1] != motor_id {
		*index = motor_ids.len();
		return Some(Err(MissingResponse { motor_id }.into()));
	}

	// Skip the per-motor CRC (2 bytes). The final motor's CRC doubles as the packet CRC and is
	// stripped while reading the packet, so it may be absent for the last block.
	*parameters = rest.get(2..).unwrap_or(&[]);

	Some(Ok(block))
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

/// A fast sync read operation that yields the unparsed bytes read from each motor.
///
/// Returned by [`Client::fast_sync_read_bytes`].
/// The entire response is read from the bus before this iterator is returned;
/// iterating it simply splits the response into the per-motor replies.
pub struct FastSyncReadBytes<'a, T: ?Sized, E> {
	/// The unparsed per-motor blocks, starting at the error byte of the first motor.
	parameters: &'a [u8],

	/// The number of data bytes in each motor block.
	count: u16,

	/// The IDs of the motors that were addressed, in reply order.
	motor_ids: &'a [u8],

	/// The index of the next motor reply to yield.
	index: usize,

	data: PhantomData<fn(&T) -> E>,
}

impl<T: ?Sized, E> core::fmt::Debug for FastSyncReadBytes<'_, T, E> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("FastSyncReadBytes")
			.field("count", &self.count)
			.field("motor_ids", &self.motor_ids)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<'a, T: ?Sized, E> FastSyncReadBytes<'a, T, E> {
	/// The number of motor replies that have not been yielded yet.
	pub fn remaining(&self) -> usize {
		self.motor_ids.len() - self.index
	}

	/// Split the next `error + motor ID + data` block off the front of the buffer, advancing the state.
	fn next_block(&mut self) -> Option<Result<&'a [u8], ReadError<E>>> {
		split_motor_block(&mut self.parameters, self.motor_ids, &mut self.index, usize::from(self.count))
	}

	/// Read the next motor reply, or [`None`] once every motor has been yielded.
	pub fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<E>>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		Some(match self.next_block()? {
			Ok(block) => decode_motor_block(block, |data| T::from(data)),
			Err(e) => Err(e),
		})
	}

	/// Read the next motor reply borrowing the data from the read buffer, or [`None`] once every motor has been yielded.
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

impl<T, E> Iterator for FastSyncReadBytes<'_, T, E>
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

/// Parse a single `error + motor ID + data` block from a fast sync read response.
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
	use super::{FastSyncRead, FastSyncReadBytes};
	use crate::{InvalidMessage, MissingResponse, ReadError, Response};
	use assert2::{assert, let_assert};
	use core::convert::Infallible;
	use core::marker::PhantomData;

	/// Build a `FastSyncRead` directly from a raw parameter buffer (the region starting at the first error byte).
	fn fast_sync_read<'a, T>(parameters: &'a [u8], count: u16, motor_ids: &'a [u8]) -> FastSyncRead<'a, T, Infallible> {
		FastSyncRead {
			parameters,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		}
	}

	/// Build a `FastSyncReadBytes` directly from a raw parameter buffer (the region starting at the first error byte).
	fn fast_sync_read_bytes<'a, T: ?Sized>(parameters: &'a [u8], count: u16, motor_ids: &'a [u8]) -> FastSyncReadBytes<'a, T, Infallible> {
		FastSyncReadBytes {
			parameters,
			count,
			motor_ids,
			index: 0,
			data: PhantomData,
		}
	}

	#[test]
	fn read_next_borrow_yields_slices() {
		let parameters = [
			0x00, 0x01, 0x34, 0x12, 0xAA, 0xBB, // motor 1: bytes [0x34, 0x12] + skipped CRC.
			0x00, 0x02, 0x78, 0x56, // motor 2: bytes [0x78, 0x56], CRC stripped as the packet CRC.
		];
		let mut iter = fast_sync_read_bytes::<[u8]>(&parameters, 2, &[1, 2]);

		let_assert!(Some(Ok(response)) = iter.read_next_borrow());
		assert!(response.motor_id == 1);
		assert!(response.data == [0x34, 0x12].as_slice());

		let_assert!(Some(Ok(response)) = iter.read_next_borrow());
		assert!(response.motor_id == 2);
		assert!(response.data == [0x78, 0x56].as_slice());

		assert!(let None = iter.read_next_borrow());
	}

	#[test]
	fn parses_each_motor_block() {
		// Two motors, 2 bytes each: [error, id, data_le, crc, crc] per block.
		// The final block's CRC is the packet CRC, which is stripped before parsing, so it is absent here.
		let parameters = [
			0x00, 0x01, 0x34, 0x12, 0xAA, 0xBB, // motor 1: 0x1234, with a (skipped) CRC.
			0x00, 0x02, 0x78, 0x56, // motor 2: 0x5678, CRC stripped as the packet CRC.
		];
		let mut iter = fast_sync_read::<u16>(&parameters, 2, &[1, 2]);

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
		let mut iter = fast_sync_read::<u16>(&parameters, 2, &[1, 2]);

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
	fn recovers_leading_motors_and_names_missing_one() {
		// Three motors expected, but the buffer only holds the first two complete blocks: motor 3 is missing.
		let parameters = [
			0x00, 0x01, 0x34, 0x12, 0xAA, 0xBB, // motor 1: 0x1234, with a (skipped) CRC.
			0x00, 0x02, 0x78, 0x56, // motor 2: 0x5678, its CRC coincides with the (stripped) packet CRC.
		];
		let mut iter = fast_sync_read::<u16>(&parameters, 2, &[1, 2, 3]);

		// The motors that did reply are yielded first.
		let_assert!(Some(Ok(response)) = iter.next());
		assert!(response.motor_id == 1 && response.data == 0x1234);
		let_assert!(Some(Ok(response)) = iter.next());
		assert!(response.motor_id == 2 && response.data == 0x5678);

		// The missing motor is named rather than silently ending iteration.
		let_assert!(Some(Err(ReadError::InvalidMessage(InvalidMessage::MissingResponse(MissingResponse { motor_id })))) = iter.next());
		assert!(motor_id == 3);
		assert!(let None = iter.next());
	}
}
