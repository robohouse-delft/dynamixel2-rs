use super::{instruction_id, packet_id};
use crate::endian::{write_u16_le, write_u32_le};
use crate::{Bus, TransferError};

/// Data for a specific motor.
///
/// Used by synchronous write commands.
pub struct SyncWriteData<T> {
	/// The motor the data is for.
	pub motor_id: u8,

	/// The data to be written to the motor.
	pub data: T,
}

impl<Stream, ReadBuffer, WriteBuffer> Bus<Stream, ReadBuffer, WriteBuffer>
where
	Stream: std::io::Read + std::io::Write,
	ReadBuffer: AsRef<[u8]> + AsMut<[u8]>,
	WriteBuffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously write an arbitrary number of bytes to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	///
	/// # Panics
	/// The amount of data to write for each motor must be exactly `count` bytes.
	/// This function panics if that is not the case.
	pub fn sync_write<'a, I>(&mut self, address: u16, count: u16, data: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = SyncWriteData<&'a [u8]>>,
		I::IntoIter: std::iter::ExactSizeIterator,
	{
		let data = data.into_iter();
		let motors = data.len();
		let stride = 1 + usize::from(count);
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			for (i, command) in data.enumerate() {
				assert!(command.data.len() == count as usize);
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				buffer[1..].copy_from_slice(command.data);
			}
		})?;
		Ok(())
	}

	/// Synchronously write a 8 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u8<I>(&mut self, address: u16, data: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = SyncWriteData<u8>>,
		I::IntoIter: std::iter::ExactSizeIterator,
	{
		let data = data.into_iter();
		let count = core::mem::size_of::<u8>();
		let motors = data.len();
		let stride = 1 + count;
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			for (i, command) in data.enumerate() {
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				buffer[1] = command.data;
			}
		})?;
		Ok(())
	}

	/// Synchronously write a 16 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u16<I>(&mut self, address: u16, data: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = SyncWriteData<u16>>,
		I::IntoIter: std::iter::ExactSizeIterator,
	{
		let data = data.into_iter();
		let count = core::mem::size_of::<u16>();
		let motors = data.len();
		let stride = 1 + count;
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			for (i, command) in data.enumerate() {
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				write_u16_le(&mut buffer[1..], command.data);
			}
		})?;
		Ok(())
	}

	/// Synchronously write a 32 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u32<I>(&mut self, address: u16, data: I) -> Result<(), TransferError>
	where
		I: IntoIterator<Item = SyncWriteData<u32>>,
		I::IntoIter: std::iter::ExactSizeIterator,
	{
		let data = data.into_iter();
		let count = core::mem::size_of::<u32>();
		let motors = data.len();
		let stride = 1 + count;
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count as u16);
			for (i, command) in data.enumerate() {
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				write_u32_le(&mut buffer[1..], command.data);
			}
		})?;
		Ok(())
	}
}
