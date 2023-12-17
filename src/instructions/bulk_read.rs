use super::{instruction_id, packet_id, BulkReadData};
use crate::endian::{write_u16_le, write_u8_le};
use crate::{Bus, ReadError, Response, TransferError, WriteError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read arbitrary data ranges from multiple motors in one command.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// The data for multi-byte registers is received in little-endian format.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub fn bulk_read_cb<Read, F>(&mut self, reads: &[Read], mut on_response: F) -> Result<(), WriteError>
	where
		Read: AsRef<BulkReadData>,
		F: FnMut(&BulkReadData, Result<Response<&[u8]>, ReadError>),
	{
		for i in 0..reads.len() {
			for j in i + 1..reads.len() {
				if reads[i].as_ref().motor_id == reads[j].as_ref().motor_id {
					panic!(
						"bulk_read_cb: motor ID {} used multiple at index {} and {}",
						reads[i].as_ref().motor_id,
						i,
						j
					)
				}
			}
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::BULK_READ, 5 * reads.len(), |buffer| {
			for (i, read) in reads.iter().enumerate() {
				let read = read.as_ref();
				let buffer = &mut buffer[i..][..5];
				write_u8_le(&mut buffer[0..], read.motor_id);
				write_u16_le(&mut buffer[1..], read.address);
				write_u16_le(&mut buffer[3..], read.count);
			}
		})?;
		for read in reads {
			let read = read.as_ref();
			let response = self.read_status_response().and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), read.motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), read.count.into())?;
				Ok(response)
			});

			match response {
				Ok(response) => on_response(read, Ok((&response).into())),
				Err(e) => on_response(read, Err(e)),
			}
		}
		Ok(())
	}

	/// Synchronously read arbitrary data ranges from multiple motors in one command.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// If this function fails to get the data from any of the motors, the entire function retrns an error.
	/// If you need access to the data from other motors, or if you want acces to the error for each motor, see [`Self::bulk_read_cb`].
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub fn bulk_read<Read>(&mut self, reads: &[Read]) -> Result<Vec<Response<Vec<u8>>>, TransferError>
	where
		Read: AsRef<BulkReadData>,
	{
		let mut responses = Vec::with_capacity(reads.len());
		let mut read_error = None;

		self.bulk_read_cb(reads, |_read, response| {
			if read_error.is_none() {
				match response {
					Err(e) => read_error = Some(e),
					Ok(response) => responses.push(Response {
						motor_id: response.motor_id,
						alert: response.alert,
						data: response.data.to_owned()
					}),
				}
			}
		})?;

		if let Some(e) = read_error {
			Err(e.into())
		} else {
			Ok(responses)
		}
	}
}
