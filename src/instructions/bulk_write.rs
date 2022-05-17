use super::{instruction_id, packet_id, BulkData};
use crate::endian::{write_u16_le, write_u8_le};
use crate::{Bus, WriteError};

#[cfg(feature = "sync")]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously write arbitrary data ranges to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	/// Unlike the sync write instruction, a bulk write allows you to write a different amount of data to a different address for each motor.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one write.
	///
	/// This function also panics if the amount of data for a motor exceeds the capacity of a u16.
	pub fn bulk_write<'a, Write>(&mut self, writes: &[Write]) -> Result<(), WriteError>
	where
		Write: AsRef<BulkData<&'a [u8]>>,
	{
		let mut parameter_count = 0;
		for write in writes {
			let write = write.as_ref();
			if write.data.len() > u16::MAX.into() {
				panic!(
					"bulk_write: data length ({}) for motor {} exceeds maximum size of {}",
					write.data.len(),
					write.motor_id,
					u16::MAX
				);
			}
			parameter_count += 5 + write.data.len();
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			let mut offset = 0;
			for write in writes {
				let write = write.as_ref();
				let buffer = &mut buffer[offset..];
				offset += 5 + write.data.len();
				write_u8_le(&mut buffer[0..], write.motor_id);
				write_u16_le(&mut buffer[1..], write.address);
				write_u16_le(&mut buffer[3..], write.data.len() as u16);
				buffer[5..][..write.data.len()].copy_from_slice(write.data);
			}
		})
	}
}

#[cfg(any(feature = "async_smol", feature = "async_tokio"))]
impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously write arbitrary data ranges to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	/// Unlike the sync write instruction, a bulk write allows you to write a different amount of data to a different address for each motor.
	///
	/// # Panics
	/// The protocol forbids specifying the same motor ID multiple times.
	/// This function panics if the same motor ID is used for more than one write.
	///
	/// This function also panics if the amount of data for a motor exceeds the capacity of a u16.
	pub async fn bulk_write<'a, Write>(&mut self, writes: &[Write]) -> Result<(), WriteError>
	where
		Write: AsRef<BulkData<&'a [u8]>>,
	{
		let mut parameter_count = 0;
		for write in writes {
			let write = write.as_ref();
			if write.data.len() > u16::MAX.into() {
				panic!(
					"bulk_write: data length ({}) for motor {} exceeds maximum size of {}",
					write.data.len(),
					write.motor_id,
					u16::MAX
				);
			}
			parameter_count += 5 + write.data.len();
		}

		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			let mut offset = 0;
			for write in writes {
				let write = write.as_ref();
				let buffer = &mut buffer[offset..];
				offset += 5 + write.data.len();
				write_u8_le(&mut buffer[0..], write.motor_id);
				write_u16_le(&mut buffer[1..], write.address);
				write_u16_le(&mut buffer[3..], write.data.len() as u16);
				buffer[5..][..write.data.len()].copy_from_slice(write.data);
			}
		})
		.await
	}
}
