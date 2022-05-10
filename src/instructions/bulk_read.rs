use super::{instruction_id, packet_id, BulkData};
use crate::endian::{write_u16_le, write_u8_le};
use crate::{Bus, ReadError, TransferError, WriteError};

pub struct BulkRead {
	pub motor_id: u8,
	pub address: u16,
	pub count: u16,
}

impl AsRef<BulkRead> for BulkRead {
	fn as_ref(&self) -> &Self {
		self
	}
}

#[cfg(feature = "sync")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read arbitrary data ranges from multiple motors in one command.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub fn bulk_read_cb<Read, F>(&mut self, reads: &[Read], mut on_response: F) -> Result<(), WriteError>
	where
		Read: AsRef<BulkRead>,
		F: FnMut(Result<BulkData<&[u8]>, ReadError>),
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

		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 5 * reads.len(), |buffer| {
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
				Ok(response) => on_response(Ok(BulkData {
					motor_id: read.motor_id,
					address: read.address,
					data: response.parameters(),
				})),
				Err(e) => on_response(Err(e)),
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
	pub fn bulk_read<Read>(&mut self, reads: &[Read]) -> Result<Vec<BulkData<Vec<u8>>>, TransferError>
	where
		Read: AsRef<BulkRead>,
	{
		let mut responses = Vec::with_capacity(reads.len());
		let mut read_error = None;

		self.bulk_read_cb(reads, |bulk_data| {
			if read_error.is_none() {
				match bulk_data {
					Err(e) => read_error = Some(e),
					Ok(bulk_data) => responses.push(BulkData {
						motor_id: bulk_data.motor_id,
						address: bulk_data.address,
						data: bulk_data.data.to_owned(),
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

#[cfg(feature = "async_smol")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously read arbitrary data ranges from multiple motors in one command.
	///
	/// Unlike the sync read instruction, a bulk read can be used to read a different amount of data from a different address for each motor.
	///
	/// The `on_response` function is called for the reply from each motor.
	/// If the function fails to write the instruction, an error is returned and the function is not called.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one read.
	pub async fn bulk_read_cb<Read, F>(&mut self, reads: &[Read], mut on_response: F) -> Result<(), WriteError>
	where
		Read: AsRef<BulkRead>,
		F: FnMut(Result<BulkData<&[u8]>, ReadError>),
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

		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_READ, 5 * reads.len(), |buffer| {
			for (i, read) in reads.iter().enumerate() {
				let read = read.as_ref();
				let buffer = &mut buffer[i..][..5];
				write_u8_le(&mut buffer[0..], read.motor_id);
				write_u16_le(&mut buffer[1..], read.address);
				write_u16_le(&mut buffer[3..], read.count);
			}
		}).await?;
		for read in reads {
			let read = read.as_ref();
			let response = self.read_status_response().await.and_then(|response| {
				crate::InvalidPacketId::check(response.packet_id(), read.motor_id)?;
				crate::InvalidParameterCount::check(response.parameters().len(), read.count.into())?;
				Ok(response)
			});

			match response {
				Ok(response) => on_response(Ok(BulkData {
					motor_id: read.motor_id,
					address: read.address,
					data: response.parameters(),
				})),
				Err(e) => on_response(Err(e)),
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
	pub async fn bulk_read<Read>(&mut self, reads: &[Read]) -> Result<Vec<BulkData<Vec<u8>>>, TransferError>
	where
		Read: AsRef<BulkRead>,
	{
		let mut responses = Vec::with_capacity(reads.len());
		let mut read_error = None;

		self.bulk_read_cb(reads, |bulk_data| {
			if read_error.is_none() {
				match bulk_data {
					Err(e) => read_error = Some(e),
					Ok(bulk_data) => responses.push(BulkData {
						motor_id: bulk_data.motor_id,
						address: bulk_data.address,
						data: bulk_data.data.to_owned(),
					}),
				}
			}
		}).await?;

		if let Some(e) = read_error {
			Err(e.into())
		} else {
			Ok(responses)
		}
	}
}
