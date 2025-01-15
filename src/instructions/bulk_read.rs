use super::{instruction_id, packet_id, BulkReadData};
use crate::bus::endian::{write_u16_le, write_u8_le};
use crate::{Client, ReadError, Response, WriteError};

use crate::bus::decode_status_packet_bytes;
use std::marker::PhantomData;

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read arbitrary data ranges from multiple motors.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub fn bulk_read_bytes<'a, T>(
		&'a mut self,
		reads: &'a [BulkReadData],
	) -> Result<BulkReadBytes<'a, T, SerialPort, Buffer>, WriteError<SerialPort::Error>>
	where
		T: for<'b> From<&'b [u8]>,
	{
		for i in 0..reads.len() {
			for j in i + 1..reads.len() {
				if reads[i].motor_id == reads[j].motor_id {
					panic!(
						"bulk_read_cb: motor ID {} used multiple at index {} and {}",
						reads[i].motor_id, i, j
					)
				}
			}
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::BULK_READ, 5 * reads.len(), |buffer| {
			for (i, read) in reads.iter().enumerate() {
				let buffer = &mut buffer[i..][..5];
				write_u8_le(&mut buffer[0..], read.motor_id);
				write_u16_le(&mut buffer[1..], read.address);
				write_u16_le(&mut buffer[3..], read.count);
			}
			Ok(())
		})?;

		Ok(BulkReadBytes {
			client: self,
			bulk_read_data: reads,
			index: 0,
			data: PhantomData,
		})
	}
}

/// A bulk read operation that returns unparsed bytes.
pub struct BulkReadBytes<'a, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	client: &'a mut Client<SerialPort, Buffer>,
	bulk_read_data: &'a [BulkReadData],
	index: usize,
	data: PhantomData<fn() -> T>,
}

impl<T, SerialPort, Buffer> core::fmt::Debug for BulkReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort + std::fmt::Debug,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("BulkRead")
			.field("serial_port", self.client.serial_port())
			.field("bulk_read_data", &self.bulk_read_data)
			.field("index", &self.index)
			.field("data", &format_args!("{}", core::any::type_name::<T>()))
			.finish()
	}
}

impl<T, SerialPort, Buffer> Drop for BulkReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	fn drop(&mut self) {
		while self.next().is_some() {}
	}
}

impl<T, SerialPort, Buffer> Iterator for BulkReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	type Item = Result<Response<T>, crate::error::ReadError<SerialPort::Error>>;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_next()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(0, Some(self.remaining()))
	}
}

impl<T, SerialPort, Buffer> BulkReadBytes<'_, T, SerialPort, Buffer>
where
	T: for<'b> From<&'b [u8]>,
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Get the number of responses that should still be received.
	pub fn remaining(&self) -> usize {
		self.bulk_read_data.len() - self.index
	}

	/// Read the next motor reply.
	pub fn read_next(&mut self) -> Option<Result<Response<T>, ReadError<SerialPort::Error>>> {
		let BulkReadData {motor_id, count, .. } = self.pop_bulk_read_data()?;
		Some(self.next_response(motor_id, count))
	}

	fn pop_bulk_read_data(&mut self) -> Option<BulkReadData> {
		let data = self.bulk_read_data.get(self.index)?;
		self.index += 1;
		Some(*data)
	}

	fn next_response(&mut self, motor_id: u8, count: u16) -> Result<Response<T>, ReadError<SerialPort::Error>> {
		let response = self.client.read_status_response(count)?;
		// TODO: Allow a response from a motor later in the list (meaning we missed an earlier motor response).
		// We need to report a timeout or something for the missed motor though.
		crate::InvalidPacketId::check(response.packet_id(), motor_id)?;
		crate::InvalidParameterCount::check(response.parameters().len(), count.into())?;

		Ok(decode_status_packet_bytes(response))
	}
}
