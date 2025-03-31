use super::{instruction_id, packet_id, SyncWriteData};
use crate::bus::endian::write_u16_le;
use crate::{Client, WriteError};

impl<SerialPort, Buffer> Client<SerialPort, Buffer>
where
	SerialPort: crate::SerialPort,
	Buffer: AsRef<[u8]> + AsMut<[u8]>,
{
	/// Synchronously write an arbitrary number of bytes to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	///
	/// # Panics
	/// The amount of data to write for each motor must be exactly `count` bytes.
	/// This function panics if that is not the case.
	///
	/// # Example
	/// ```no_run
	/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// use dynamixel2::Client;
	/// use dynamixel2::instructions::SyncWriteData;
	/// use std::time::Duration;
	///
	/// let mut client = Client::open("/dev/ttyUSB0", 57600)?;
	/// // Write to register 116 of motor 1 and and 2 at the same time.
	/// client.sync_write_bytes(116, 4, &[
	///   SyncWriteData {
	///     motor_id: 1,
	///     data: 2000u32.to_le_bytes(),
	///   },
	///   SyncWriteData {
	///     motor_id: 2,
	///     data: 1600u32.to_le_bytes(),
	///   },
	/// ])?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn sync_write_bytes<'a, Iter, Data, Buf>(
		&mut self,
		address: u16,
		count: u16,
		data: Iter,
	) -> Result<(), WriteError<SerialPort::Error>>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: ExactSizeIterator,
		Data: AsRef<SyncWriteData<Buf>>,
		Buf: AsRef<[u8]> + 'a,
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
				assert_eq!(command.data.as_ref().len(), count as usize);
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				buffer[1..].copy_from_slice(command.data.as_ref());
			}
			Ok(())
		})
	}

	/// Synchronously write a value to multiple motors.
	///
	/// Each motor will perform the write as soon as it receives the command.
	/// This gives much shorter delays than executing a regular [`Self::write`] for each motor individually.
	pub fn sync_write<Iter, Data, T>(&mut self, address: u16, data: Iter) -> Result<(), WriteError<SerialPort::Error>>
	where
		Iter: IntoIterator<Item = Data>,
		Iter::IntoIter: ExactSizeIterator,
		Data: AsRef<SyncWriteData<T>>,
		T: crate::bus::Data,
	{
		let data = data.into_iter();
		let count = T::ENCODED_SIZE;
		let motors = data.len();
		let stride = 1 + count as usize;
		let parameter_count = 4 + motors * stride;
		self.write_instruction(packet_id::BROADCAST, instruction_id::SYNC_WRITE, parameter_count, |buffer| {
			write_u16_le(&mut buffer[0..], address);
			write_u16_le(&mut buffer[2..], count);
			for (i, command) in data.enumerate() {
				let command = command.as_ref();
				let buffer = &mut buffer[4 + i * stride..][..stride];
				buffer[0] = command.motor_id;
				command.data.encode(&mut buffer[1..])?;
			}
			Ok(())
		})
	}
}
