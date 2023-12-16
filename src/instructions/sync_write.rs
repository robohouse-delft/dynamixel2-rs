use super::{instruction_id, packet_id, SyncWriteData};
use crate::endian::{write_u16_le, write_u32_le};
use crate::{Bus, WriteError};

impl<ReadBuffer, WriteBuffer> Bus<ReadBuffer, WriteBuffer>
where
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
	pub fn sync_write<'a, Iter, Data>(&mut self, address: u16, count: u16, data: Iter) -> Result<(), WriteError>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: std::iter::ExactSizeIterator,
		Data: AsRef<SyncWriteData<&'a [u8]>>,
	{
		let data = data.into_iter();
		let motors = data.len();
		let stride = 1 + usize::from(count);
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			for (i, command) in data.enumerate() {
				let command = command.as_ref();
				assert!(command.data.len() == count as usize);
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				buffer[1..].copy_from_slice(command.data);
			}
		})
	}

	/// Synchronously write a 8 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u8<Iter, Data>(&mut self, address: u16, data: Iter) -> Result<(), WriteError>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: std::iter::ExactSizeIterator,
		Data: AsRef<SyncWriteData<u8>>,
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
				let command = command.as_ref();
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				buffer[1] = command.data;
			}
		})
	}

	/// Synchronously write a 16 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u16<Iter, Data>(&mut self, address: u16, data: Iter) -> Result<(), WriteError>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: std::iter::ExactSizeIterator,
		Data: AsRef<SyncWriteData<u16>>,
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
				let command = command.as_ref();
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				write_u16_le(&mut buffer[1..], command.data);
			}
		})
	}

	/// Synchronously write a 32 bit value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write_u32<Iter, Data>(&mut self, address: u16, data: Iter) -> Result<(), WriteError>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: std::iter::ExactSizeIterator,
		Data: AsRef<SyncWriteData<u32>>,
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
				let command = command.as_ref();
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				write_u32_le(&mut buffer[1..], command.data);
			}
		})
	}
}
